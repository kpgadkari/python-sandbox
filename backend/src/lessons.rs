use axum::{
    extract::{Path as AxumPath, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    auth::require_user,
    error::ApiError,
    models::{CheckLessonRequest, CheckLessonResponse, LessonDetail, LessonSummary},
    state::AppState,
};

pub(crate) async fn list_lessons(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<LessonSummary>>, ApiError> {
    require_user(&state, &headers).await?;
    let rows = sqlx::query_as::<_, LessonSummary>(
        "SELECT id, title, prompt, description, difficulty FROM lessons WHERE is_published = 1 ORDER BY sort_order ASC",
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
    require_user(&state, &headers).await?;
    let lesson = sqlx::query_as::<_, LessonDetail>(
            "SELECT id, title, prompt, description, hint, difficulty, starter_code, expected_stdout FROM lessons WHERE id = ? AND is_published = 1",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(ApiError::not_found("lesson not found"))?;
    Ok(Json(lesson))
}

pub(crate) async fn check_lesson(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<CheckLessonRequest>,
) -> Result<Json<CheckLessonResponse>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let expected =
        sqlx::query_scalar::<_, String>("SELECT expected_stdout FROM lessons WHERE id = ?")
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
    Ok(Json(CheckLessonResponse {
        passed,
        expected_stdout: expected,
    }))
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
}
