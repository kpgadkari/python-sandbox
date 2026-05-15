use std::{
    collections::HashMap,
    env, fs,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Path as AxumPath, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use cookie::{Cookie, SameSite};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

const SESSION_COOKIE: &str = "sandbox_session";
const MAX_PROJECT_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let data_dir = PathBuf::from(env::var("SANDBOX_DATA_DIR").unwrap_or_else(|_| "./data".into()));
    fs::create_dir_all(data_dir.join("projects")).context("create data directories")?;

    let db_path = PathBuf::from(
        env::var("SANDBOX_DB_PATH")
            .unwrap_or_else(|_| data_dir.join("sandbox.db").display().to_string()),
    );
    let connection = Connection::open(db_path).context("open sqlite database")?;
    init_db(&connection)?;
    seed_user(&connection)?;
    seed_lessons(&connection)?;

    let state = AppState {
        db: Arc::new(Mutex::new(connection)),
        data_dir,
    };

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/:id", get(get_project).delete(delete_project))
        .route("/api/projects/:id/files", put(save_project_files))
        .route("/api/lessons", get(list_lessons))
        .route("/api/lessons/:id", get(get_lesson))
        .route("/api/lessons/:id/check", post(check_lesson))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let bind = env::var("SANDBOX_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let addr: SocketAddr = bind.parse().context("parse SANDBOX_BIND")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}

fn init_db(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            display_name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS lessons (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            prompt TEXT NOT NULL,
            starter_code TEXT NOT NULL,
            expected_stdout TEXT NOT NULL,
            hidden_tests TEXT NOT NULL DEFAULT '[]',
            sort_order INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS attempts (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            lesson_id TEXT NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
            code_snapshot TEXT NOT NULL,
            stdout TEXT NOT NULL,
            passed INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

fn seed_user(conn: &Connection) -> anyhow::Result<()> {
    let username = env::var("SANDBOX_USERNAME").unwrap_or_else(|_| "parent".into());
    let password = env::var("SANDBOX_PASSWORD").unwrap_or_else(|_| "change-me".into());
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM users WHERE username = ?",
            [&username],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_some() {
        return Ok(());
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("hash password: {err}"))?
        .to_string();
    conn.execute(
        "INSERT INTO users (id, username, password_hash, display_name, created_at) VALUES (?, ?, ?, ?, ?)",
        params![Uuid::new_v4().to_string(), username, password_hash, "Home Coder", Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn seed_lessons(conn: &Connection) -> anyhow::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM lessons", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let lessons = [
        (
            "hello-python",
            "Hello, Python",
            "Print a friendly greeting.",
            "print(\"hello, python\")\n",
            "hello, python\n",
        ),
        (
            "name-input",
            "Ask for a Name",
            "Ask for a name and print a greeting. Try running it and answering the prompt.",
            "name = input(\"Name? \")\nprint(\"Hi\", name)\n",
            "Hi Ada\n",
        ),
        (
            "tiny-loop",
            "Tiny Loop",
            "Use a loop to print the numbers 1 through 3.",
            "for n in range(1, 4):\n    print(n)\n",
            "1\n2\n3\n",
        ),
    ];

    for (index, lesson) in lessons.iter().enumerate() {
        conn.execute(
            "INSERT INTO lessons (id, title, prompt, starter_code, expected_stdout, hidden_tests, sort_order)
             VALUES (?, ?, ?, ?, ?, '[]', ?)",
            params![lesson.0, lesson.1, lesson.2, lesson.3, lesson.4, index as i64],
        )?;
    }
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    let user = conn
        .query_row(
            "SELECT id, username, password_hash, display_name FROM users WHERE username = ?",
            [&request.username],
            |row| {
                Ok(UserWithHash {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    display_name: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or(ApiError::unauthorized())?;

    {
        let parsed_hash =
            PasswordHash::new(&user.password_hash).map_err(|_| ApiError::unauthorized())?;
        Argon2::default()
            .verify_password(request.password.as_bytes(), &parsed_hash)
            .map_err(|_| ApiError::unauthorized())?;
    }

    let token = random_token();
    let now = Utc::now();
    let expires = now + chrono::Duration::days(30);
    conn.execute(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
        params![&token, &user.id, now.to_rfc3339(), expires.to_rfc3339()],
    )?;

    let cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::days(30))
        .build();

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).map_err(|_| ApiError::internal("cookie"))?,
    );

    Ok((
        headers,
        Json(MeResponse {
            user: user.into_public(),
        }),
    )
        .into_response())
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    if let Some(token) = session_token(&headers) {
        let conn = state
            .db
            .lock()
            .map_err(|_| ApiError::internal("database lock"))?;
        conn.execute("DELETE FROM sessions WHERE token = ?", [token])?;
    }

    let cookie = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .build();
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).map_err(|_| ApiError::internal("cookie"))?,
    );
    Ok(response)
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, ApiError> {
    let user = require_user(&state, &headers)?;
    Ok(Json(MeResponse { user }))
}

async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProjectSummary>>, ApiError> {
    let user = require_user(&state, &headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at FROM projects WHERE owner_id = ? ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([user.id], |row| {
        Ok(ProjectSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;
    Ok(Json(rows.collect::<Result<Vec<_>, _>>()?))
}

async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<ProjectDetail>, ApiError> {
    let user = require_user(&state, &headers)?;
    let project_id = Uuid::new_v4().to_string();
    let title = clean_title(request.title);
    let now = Utc::now().to_rfc3339();
    let mut files = HashMap::new();
    files.insert(
        "main.py".to_string(),
        request
            .starter_code
            .unwrap_or_else(|| "print(\"hello, python\")\n".to_string()),
    );
    validate_files(&files)?;
    write_project_files(&state.data_dir, &project_id, &files)?;

    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    conn.execute(
        "INSERT INTO projects (id, owner_id, title, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        params![&project_id, &user.id, &title, &now, &now],
    )?;

    Ok(Json(ProjectDetail {
        id: project_id,
        title,
        created_at: now.clone(),
        updated_at: now,
        files,
    }))
}

async fn get_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ProjectDetail>, ApiError> {
    let user = require_user(&state, &headers)?;
    let summary = project_for_owner(&state, &id, &user.id)?;
    let files = read_project_files(&state.data_dir, &id)?;
    Ok(Json(ProjectDetail {
        id: summary.id,
        title: summary.title,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
        files,
    }))
}

async fn save_project_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<SaveFilesRequest>,
) -> Result<Json<ProjectDetail>, ApiError> {
    let user = require_user(&state, &headers)?;
    let mut summary = project_for_owner(&state, &id, &user.id)?;
    validate_files(&request.files)?;
    write_project_files(&state.data_dir, &id, &request.files)?;

    let now = Utc::now().to_rfc3339();
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    conn.execute(
        "UPDATE projects SET updated_at = ? WHERE id = ? AND owner_id = ?",
        params![&now, &id, &user.id],
    )?;
    summary.updated_at = now;

    Ok(Json(ProjectDetail {
        id: summary.id,
        title: summary.title,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
        files: request.files,
    }))
}

async fn delete_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let user = require_user(&state, &headers)?;
    project_for_owner(&state, &id, &user.id)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    conn.execute(
        "DELETE FROM projects WHERE id = ? AND owner_id = ?",
        params![&id, &user.id],
    )?;
    let project_dir = state.data_dir.join("projects").join(&id);
    let _ = fs::remove_dir_all(project_dir);
    Ok(StatusCode::NO_CONTENT)
}

async fn list_lessons(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LessonSummary>>, ApiError> {
    require_user(&state, &headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    let mut stmt = conn.prepare("SELECT id, title, prompt FROM lessons ORDER BY sort_order ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(LessonSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            prompt: row.get(2)?,
        })
    })?;
    Ok(Json(rows.collect::<Result<Vec<_>, _>>()?))
}

async fn get_lesson(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<LessonDetail>, ApiError> {
    require_user(&state, &headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    let lesson = conn
        .query_row(
            "SELECT id, title, prompt, starter_code, expected_stdout FROM lessons WHERE id = ?",
            [id],
            |row| {
                Ok(LessonDetail {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    prompt: row.get(2)?,
                    starter_code: row.get(3)?,
                    expected_stdout: row.get(4)?,
                })
            },
        )
        .optional()?
        .ok_or(ApiError::not_found("lesson not found"))?;
    Ok(Json(lesson))
}

async fn check_lesson(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<CheckLessonRequest>,
) -> Result<Json<CheckLessonResponse>, ApiError> {
    let user = require_user(&state, &headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    let expected: String = conn
        .query_row(
            "SELECT expected_stdout FROM lessons WHERE id = ?",
            [&id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(ApiError::not_found("lesson not found"))?;
    let passed = normalize_stdout(&request.stdout) == normalize_stdout(&expected);
    conn.execute(
        "INSERT INTO attempts (id, user_id, lesson_id, code_snapshot, stdout, passed, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            Uuid::new_v4().to_string(),
            user.id,
            id,
            request.code_snapshot,
            request.stdout,
            if passed { 1 } else { 0 },
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(Json(CheckLessonResponse {
        passed,
        expected_stdout: expected,
    }))
}

fn require_user(state: &AppState, headers: &HeaderMap) -> Result<PublicUser, ApiError> {
    let token = session_token(headers).ok_or(ApiError::unauthorized())?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    conn.query_row(
        "SELECT users.id, users.username, users.display_name
         FROM sessions JOIN users ON users.id = sessions.user_id
         WHERE sessions.token = ? AND sessions.expires_at > ?",
        params![token, Utc::now().to_rfc3339()],
        |row| {
            Ok(PublicUser {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
            })
        },
    )
    .optional()?
    .ok_or(ApiError::unauthorized())
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == SESSION_COOKIE).then(|| value.to_string())
    })
}

fn project_for_owner(
    state: &AppState,
    project_id: &str,
    owner_id: &str,
) -> Result<ProjectSummary, ApiError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    conn.query_row(
        "SELECT id, title, created_at, updated_at FROM projects WHERE id = ? AND owner_id = ?",
        params![project_id, owner_id],
        |row| {
            Ok(ProjectSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or(ApiError::not_found("project not found"))
}

fn write_project_files(
    data_dir: &Path,
    project_id: &str,
    files: &HashMap<String, String>,
) -> Result<(), ApiError> {
    let project_dir = data_dir.join("projects").join(project_id);
    if project_dir.exists() {
        fs::remove_dir_all(&project_dir)
            .map_err(|_| ApiError::internal("clear project directory"))?;
    }
    fs::create_dir_all(&project_dir).map_err(|_| ApiError::internal("create project directory"))?;
    for (path, contents) in files {
        let safe_path = safe_relative_path(path)?;
        let target = project_dir.join(safe_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| ApiError::internal("create file directory"))?;
        }
        fs::write(target, contents).map_err(|_| ApiError::internal("write project file"))?;
    }
    Ok(())
}

fn read_project_files(
    data_dir: &Path,
    project_id: &str,
) -> Result<HashMap<String, String>, ApiError> {
    let project_dir = data_dir.join("projects").join(project_id);
    let mut files = HashMap::new();
    read_files_recursive(&project_dir, &project_dir, &mut files)?;
    Ok(files)
}

fn read_files_recursive(
    base: &Path,
    dir: &Path,
    files: &mut HashMap<String, String>,
) -> Result<(), ApiError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|_| ApiError::internal("read project directory"))? {
        let entry = entry.map_err(|_| ApiError::internal("read project entry"))?;
        let path = entry.path();
        if path.is_dir() {
            read_files_recursive(base, &path, files)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(|_| ApiError::internal("project path"))?
                .to_string_lossy()
                .replace('\\', "/");
            let contents =
                fs::read_to_string(&path).map_err(|_| ApiError::internal("read project file"))?;
            files.insert(rel, contents);
        }
    }
    Ok(())
}

fn validate_files(files: &HashMap<String, String>) -> Result<(), ApiError> {
    if !files.contains_key("main.py") {
        return Err(ApiError::bad_request("project must include main.py"));
    }
    let total: usize = files
        .iter()
        .map(|(path, contents)| path.len() + contents.len())
        .sum();
    if total > MAX_PROJECT_BYTES {
        return Err(ApiError::bad_request("project is larger than 1 MB"));
    }
    for path in files.keys() {
        safe_relative_path(path)?;
    }
    Ok(())
}

fn safe_relative_path(path: &str) -> Result<PathBuf, ApiError> {
    let candidate = Path::new(path);
    if candidate.is_absolute() || path.is_empty() {
        return Err(ApiError::bad_request("invalid file path"));
    }
    let mut safe = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            _ => return Err(ApiError::bad_request("invalid file path")),
        }
    }
    Ok(safe)
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn clean_title(title: Option<String>) -> String {
    let title = title.unwrap_or_else(|| "Untitled Project".into());
    let trimmed = title.trim();
    if trimmed.is_empty() {
        "Untitled Project".into()
    } else {
        trimmed.chars().take(80).collect()
    }
}

fn normalize_stdout(value: &str) -> String {
    value.replace("\r\n", "\n").trim_end().to_string()
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("{message}")]
    Http { status: StatusCode, message: String },
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}

impl ApiError {
    fn unauthorized() -> Self {
        Self::Http {
            status: StatusCode::UNAUTHORIZED,
            message: "unauthorized".into(),
        }
    }

    fn not_found(message: &str) -> Self {
        Self::Http {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn bad_request(message: &str) -> Self {
        Self::Http {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: &str) -> Self {
        Self::Http {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Http { status, message } => {
                (status, Json(ErrorResponse { error: message })).into_response()
            }
            ApiError::Sql(err) => {
                tracing::error!("database error: {err}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "database error".into(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct MeResponse {
    user: PublicUser,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Clone, Serialize)]
struct PublicUser {
    id: String,
    username: String,
    display_name: String,
}

struct UserWithHash {
    id: String,
    username: String,
    password_hash: String,
    display_name: String,
}

impl UserWithHash {
    fn into_public(self) -> PublicUser {
        PublicUser {
            id: self.id,
            username: self.username,
            display_name: self.display_name,
        }
    }
}

#[derive(Serialize)]
struct ProjectSummary {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct ProjectDetail {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    files: HashMap<String, String>,
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    title: Option<String>,
    starter_code: Option<String>,
}

#[derive(Deserialize)]
struct SaveFilesRequest {
    files: HashMap<String, String>,
}

#[derive(Serialize)]
struct LessonSummary {
    id: String,
    title: String,
    prompt: String,
}

#[derive(Serialize)]
struct LessonDetail {
    id: String,
    title: String,
    prompt: String,
    starter_code: String,
    expected_stdout: String,
}

#[derive(Deserialize)]
struct CheckLessonRequest {
    code_snapshot: String,
    stdout: String,
}

#[derive(Serialize)]
struct CheckLessonResponse {
    passed: bool,
    expected_stdout: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        assert!(safe_relative_path("../secret.txt").is_err());
        assert!(safe_relative_path("/secret.txt").is_err());
        assert!(safe_relative_path("nested/main.py").is_ok());
    }

    #[test]
    fn requires_main_py_and_size_limit() {
        let mut files = HashMap::new();
        files.insert("notes.txt".to_string(), "hello".to_string());
        assert!(validate_files(&files).is_err());

        files.insert("main.py".to_string(), "print(1)\n".to_string());
        assert!(validate_files(&files).is_ok());
    }
}
