use axum::{
    extract::{Path as AxumPath, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
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
    require_user(&state, &headers)?;
    let conn = state
        .db
        .lock()
        .map_err(|_| ApiError::internal("database lock"))?;
    let mut stmt = conn.prepare(
        "SELECT id, title, prompt, description, difficulty FROM lessons WHERE is_published = 1 ORDER BY sort_order ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(LessonSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            prompt: row.get(2)?,
            description: row.get(3)?,
            difficulty: row.get(4)?,
        })
    })?;
    Ok(Json(rows.collect::<Result<Vec<_>, _>>()?))
}

pub(crate) async fn get_lesson(
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
            "SELECT id, title, prompt, description, hint, difficulty, starter_code, expected_stdout FROM lessons WHERE id = ? AND is_published = 1",
            [id],
            |row| {
                Ok(LessonDetail {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    prompt: row.get(2)?,
                    description: row.get(3)?,
                    hint: row.get(4)?,
                    difficulty: row.get(5)?,
                    starter_code: row.get(6)?,
                    expected_stdout: row.get(7)?,
                })
            },
        )
        .optional()?
        .ok_or(ApiError::not_found("lesson not found"))?;
    Ok(Json(lesson))
}

pub(crate) async fn check_lesson(
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
