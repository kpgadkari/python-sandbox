use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use cookie::{Cookie, SameSite};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, OptionalExtension};

use crate::{
    error::ApiError,
    models::{LoginRequest, MeResponse, PublicUser, UserWithHash},
    state::AppState,
};

const SESSION_COOKIE: &str = "sandbox_session";

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    let user = conn
        .query_row(
            "SELECT id, username, password_hash, display_name, role FROM users WHERE username = ?",
            [&request.username],
            |row| {
                Ok(UserWithHash {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    display_name: row.get(3)?,
                    role: row.get(4)?,
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

pub(crate) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
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

pub(crate) async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, ApiError> {
    let user = require_user(&state, &headers)?;
    Ok(Json(MeResponse { user }))
}

pub(crate) fn require_user(state: &AppState, headers: &HeaderMap) -> Result<PublicUser, ApiError> {
    let token = session_token(headers).ok_or(ApiError::unauthorized())?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    conn.query_row(
        "SELECT users.id, users.username, users.display_name, users.role
         FROM sessions JOIN users ON users.id = sessions.user_id
         WHERE sessions.token = ? AND sessions.expires_at > ?",
        params![token, Utc::now().to_rfc3339()],
        |row| {
            Ok(PublicUser {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                role: row.get(3)?,
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

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::http::StatusCode;
    use rusqlite::Connection;

    use super::*;
    use crate::db::init_db;

    #[test]
    fn require_user_uses_session_cookie_and_expiry() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_db(&conn)?;
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                "user-1",
                "parent",
                "hash",
                "Home Coder",
                Utc::now().to_rfc3339()
            ],
        )?;
        conn.execute(
            "INSERT INTO sessions (token, user_id, created_at, expires_at)
             VALUES (?, ?, ?, ?)",
            params![
                "valid-token",
                "user-1",
                Utc::now().to_rfc3339(),
                (Utc::now() + chrono::Duration::days(1)).to_rfc3339()
            ],
        )?;
        conn.execute(
            "INSERT INTO sessions (token, user_id, created_at, expires_at)
             VALUES (?, ?, ?, ?)",
            params![
                "expired-token",
                "user-1",
                Utc::now().to_rfc3339(),
                (Utc::now() - chrono::Duration::days(1)).to_rfc3339()
            ],
        )?;
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
            data_dir: tempfile::tempdir()?.path().to_path_buf(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("other=ignored; sandbox_session=valid-token"),
        );
        let user = require_user(&state, &headers)?;
        assert_eq!(user.id, "user-1");
        assert_eq!(user.display_name, "Home Coder");

        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("sandbox_session=expired-token"),
        );
        assert!(matches!(
            require_user(&state, &headers),
            Err(ApiError::Http {
                status: StatusCode::UNAUTHORIZED,
                ..
            })
        ));
        Ok(())
    }
}
