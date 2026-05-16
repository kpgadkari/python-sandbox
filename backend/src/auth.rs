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
    let user = sqlx::query_as::<_, UserWithHash>(
        "SELECT id, username, password_hash, display_name, role FROM users WHERE username = ?",
    )
    .bind(&request.username)
    .fetch_optional(&state.db)
    .await?
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
    sqlx::query(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&token)
    .bind(&user.id)
    .bind(now.to_rfc3339())
    .bind(expires.to_rfc3339())
    .execute(&state.db)
    .await?;

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
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&state.db)
            .await?;
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
    let user = require_user(&state, &headers).await?;
    Ok(Json(MeResponse { user }))
}

pub(crate) async fn require_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<PublicUser, ApiError> {
    let token = session_token(headers).ok_or(ApiError::unauthorized())?;
    sqlx::query_as::<_, PublicUser>(
        "SELECT users.id, users.username, users.display_name, users.role
         FROM sessions JOIN users ON users.id = sessions.user_id
         WHERE sessions.token = ? AND sessions.expires_at > ?",
    )
    .bind(token)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(&state.db)
    .await?
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
