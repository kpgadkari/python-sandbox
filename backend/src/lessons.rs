use axum::{
    extract::{Path as AxumPath, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    auth::{require_parent, require_user},
    error::ApiError,
    models::{
        CheckLessonRequest, CheckLessonResponse, CreateLessonRequest, LessonDetail,
        LessonManageDetail, LessonManageSummary, LessonSummary, UpdateLessonRequest,
    },
    state::AppState,
};

pub(crate) async fn list_lessons(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LessonSummary>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let rows = if user.role == "parent" {
        sqlx::query_as::<_, LessonSummary>(
            "SELECT id, title, prompt, description, difficulty FROM lessons ORDER BY sort_order ASC",
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, LessonSummary>(
            "SELECT id, title, prompt, description, difficulty FROM lessons WHERE is_published = 1 ORDER BY sort_order ASC",
        )
        .fetch_all(&state.db)
        .await?
    };
    Ok(Json(rows))
}

pub(crate) async fn list_lessons_manage(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LessonManageSummary>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let rows = sqlx::query_as::<_, LessonManageSummary>(
        "SELECT id, title, prompt, description, difficulty, is_published, sort_order
         FROM lessons ORDER BY sort_order ASC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub(crate) async fn get_lesson(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<LessonDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let lesson = if user.role == "parent" {
        sqlx::query_as::<_, LessonDetail>(
            "SELECT id, title, prompt, description, hint, difficulty, starter_code FROM lessons WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, LessonDetail>(
            "SELECT id, title, prompt, description, hint, difficulty, starter_code FROM lessons WHERE id = ? AND is_published = 1",
        )
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
    }
    .ok_or(ApiError::not_found("lesson not found"))?;
    Ok(Json(lesson))
}

pub(crate) async fn get_lesson_manage(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<LessonManageDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let lesson = sqlx::query_as::<_, LessonManageDetail>(
        "SELECT id, title, prompt, description, hint, difficulty, starter_code, expected_stdout,
                hidden_tests, is_published, sort_order
         FROM lessons WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::not_found("lesson not found"))?;
    Ok(Json(lesson))
}

pub(crate) async fn create_lesson(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateLessonRequest>,
) -> Result<Json<LessonManageDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let id = clean_lesson_id(&request.id)?;
    if request.title.trim().is_empty() {
        return Err(ApiError::bad_request("title is required"));
    }
    if request.prompt.trim().is_empty() {
        return Err(ApiError::bad_request("prompt is required"));
    }
    if request.expected_stdout.trim().is_empty() {
        return Err(ApiError::bad_request("expected_stdout is required"));
    }
    let sort_order = match request.sort_order {
        Some(order) => order,
        None => {
            sqlx::query_scalar::<_, i32>("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM lessons")
                .fetch_one(&state.db)
                .await?
        }
    };
    let difficulty = request
        .difficulty
        .unwrap_or_else(|| "Beginner".into())
        .trim()
        .chars()
        .take(64)
        .collect::<String>();
    let hidden_tests = request.hidden_tests.unwrap_or_else(|| "[]".into());
    validate_hidden_tests(&hidden_tests)?;
    let is_published = request.is_published.unwrap_or(true);

    let lesson = LessonManageDetail {
        id,
        title: request.title.trim().to_string(),
        prompt: request.prompt.trim().to_string(),
        description: request.description.trim().to_string(),
        hint: request.hint.trim().to_string(),
        difficulty,
        starter_code: request.starter_code,
        expected_stdout: request.expected_stdout,
        hidden_tests,
        is_published,
        sort_order,
    };

    sqlx::query(
        "INSERT INTO lessons (id, title, prompt, description, hint, difficulty, starter_code,
                              expected_stdout, hidden_tests, is_published, sort_order)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&lesson.id)
    .bind(&lesson.title)
    .bind(&lesson.prompt)
    .bind(&lesson.description)
    .bind(&lesson.hint)
    .bind(&lesson.difficulty)
    .bind(&lesson.starter_code)
    .bind(&lesson.expected_stdout)
    .bind(&lesson.hidden_tests)
    .bind(lesson.is_published)
    .bind(lesson.sort_order)
    .execute(&state.db)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(db_err) = &err {
            if db_err.code().as_deref() == Some("23000") {
                return ApiError::bad_request("lesson id already exists");
            }
        }
        ApiError::from(err)
    })?;

    Ok(Json(lesson))
}

pub(crate) async fn update_lesson(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<UpdateLessonRequest>,
) -> Result<Json<LessonManageDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let existing = sqlx::query_as::<_, LessonManageDetail>(
        "SELECT id, title, prompt, description, hint, difficulty, starter_code, expected_stdout,
                hidden_tests, is_published, sort_order
         FROM lessons WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::not_found("lesson not found"))?;

    let title = request
        .title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.title);
    let prompt = request
        .prompt
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.prompt);
    let description = request
        .description
        .unwrap_or(existing.description)
        .trim()
        .to_string();
    let hint = request.hint.unwrap_or(existing.hint).trim().to_string();
    let difficulty = request
        .difficulty
        .map(|value| value.trim().chars().take(64).collect())
        .filter(|value: &String| !value.is_empty())
        .unwrap_or(existing.difficulty);
    let starter_code = request.starter_code.unwrap_or(existing.starter_code);
    let expected_stdout = request
        .expected_stdout
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.expected_stdout);
    let hidden_tests = request.hidden_tests.unwrap_or(existing.hidden_tests);
    validate_hidden_tests(&hidden_tests)?;
    let is_published = request.is_published.unwrap_or(existing.is_published);
    let sort_order = request.sort_order.unwrap_or(existing.sort_order);

    let lesson = LessonManageDetail {
        id,
        title,
        prompt,
        description,
        hint,
        difficulty,
        starter_code,
        expected_stdout,
        hidden_tests,
        is_published,
        sort_order,
    };

    sqlx::query(
        "UPDATE lessons SET title = ?, prompt = ?, description = ?, hint = ?, difficulty = ?,
                            starter_code = ?, expected_stdout = ?, hidden_tests = ?,
                            is_published = ?, sort_order = ?
         WHERE id = ?",
    )
    .bind(&lesson.title)
    .bind(&lesson.prompt)
    .bind(&lesson.description)
    .bind(&lesson.hint)
    .bind(&lesson.difficulty)
    .bind(&lesson.starter_code)
    .bind(&lesson.expected_stdout)
    .bind(&lesson.hidden_tests)
    .bind(lesson.is_published)
    .bind(lesson.sort_order)
    .bind(&lesson.id)
    .execute(&state.db)
    .await?;

    Ok(Json(lesson))
}

pub(crate) async fn check_lesson(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<CheckLessonRequest>,
) -> Result<Json<CheckLessonResponse>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let expected = sqlx::query_scalar::<_, String>(
        "SELECT expected_stdout FROM lessons WHERE id = ? AND is_published = 1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::not_found("lesson not found"))?;
    let passed = normalize_stdout(&request.stdout) == normalize_stdout(&expected);
    sqlx::query(
        "INSERT INTO attempts (id, user_id, lesson_id, code_snapshot, stdout, passed, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user.id)
    .bind(id)
    .bind(request.code_snapshot)
    .bind(request.stdout)
    .bind(passed)
    .bind(Utc::now().naive_utc())
    .execute(&state.db)
    .await?;
    Ok(Json(CheckLessonResponse { passed }))
}

fn clean_lesson_id(raw: &str) -> Result<String, ApiError> {
    let id = raw.trim().to_lowercase();
    if id.is_empty() {
        return Err(ApiError::bad_request("id is required"));
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(ApiError::bad_request(
            "id must use lowercase letters, numbers, and hyphens",
        ));
    }
    if id.len() > 191 {
        return Err(ApiError::bad_request("id is too long"));
    }
    Ok(id)
}

fn validate_hidden_tests(raw: &str) -> Result<(), ApiError> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(_)) => Ok(()),
        _ => Err(ApiError::bad_request("hidden_tests must be a JSON array")),
    }
}

fn normalize_stdout(value: &str) -> String {
    value.replace("\r\n", "\n").trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_stdout() {
        assert_eq!(normalize_stdout("hello\r\n"), "hello");
        assert_eq!(normalize_stdout("hello\n\n"), "hello");
        assert_eq!(normalize_stdout("hello\nworld\n"), "hello\nworld");
    }

    #[test]
    fn validates_lesson_ids() {
        assert_eq!(clean_lesson_id("hello-python").unwrap(), "hello-python");
        assert!(clean_lesson_id("").is_err());
        assert!(clean_lesson_id("Bad ID").is_err());
    }

    #[test]
    fn validates_hidden_tests() {
        assert!(validate_hidden_tests("[]").is_ok());
        assert!(validate_hidden_tests(r#"[{"in": "1"}]"#).is_ok());
        assert!(validate_hidden_tests("{}").is_err());
        assert!(validate_hidden_tests("not json").is_err());
    }
}
