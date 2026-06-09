use axum::{
    extract::{Path as AxumPath, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    auth::{require_child, require_parent, require_user},
    error::ApiError,
    models::{
        CreateSubmissionRequest, ReviewSubmissionRequest, SubmissionDetail, SubmissionSummary,
    },
    state::AppState,
};

pub(crate) async fn list_submissions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SubmissionSummary>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let rows = if user.role == "parent" {
        sqlx::query_as::<_, SubmissionSummary>(
            "SELECT s.id, s.lesson_id, l.title AS lesson_title, u.display_name AS submitter_name,
                    s.status, s.note, s.parent_feedback,
                    DATE_FORMAT(s.created_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS created_at,
                    DATE_FORMAT(s.reviewed_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS reviewed_at
             FROM submissions s
             JOIN lessons l ON l.id = s.lesson_id
             JOIN users u ON u.id = s.user_id
             ORDER BY s.created_at DESC",
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, SubmissionSummary>(
            "SELECT s.id, s.lesson_id, l.title AS lesson_title, u.display_name AS submitter_name,
                    s.status, s.note, s.parent_feedback,
                    DATE_FORMAT(s.created_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS created_at,
                    DATE_FORMAT(s.reviewed_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS reviewed_at
             FROM submissions s
             JOIN lessons l ON l.id = s.lesson_id
             JOIN users u ON u.id = s.user_id
             WHERE s.user_id = ?
             ORDER BY s.created_at DESC",
        )
        .bind(&user.id)
        .fetch_all(&state.db)
        .await?
    };
    Ok(Json(rows))
}

pub(crate) async fn create_submission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateSubmissionRequest>,
) -> Result<Json<SubmissionDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_child(&user)?;
    let lesson_id = request.lesson_id.trim();
    if lesson_id.is_empty() {
        return Err(ApiError::bad_request("lesson_id is required"));
    }
    if request.code_snapshot.trim().is_empty() {
        return Err(ApiError::bad_request("code_snapshot is required"));
    }
    let lesson_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM lessons WHERE id = ? AND is_published = 1",
    )
    .bind(lesson_id)
    .fetch_one(&state.db)
    .await?;
    if lesson_exists == 0 {
        return Err(ApiError::not_found("lesson not found"));
    }

    let submission_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    let note = request
        .note
        .unwrap_or_default()
        .trim()
        .chars()
        .take(500)
        .collect::<String>();
    sqlx::query(
        "INSERT INTO submissions (id, user_id, lesson_id, code_snapshot, stdout, note, status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&submission_id)
    .bind(&user.id)
    .bind(lesson_id)
    .bind(&request.code_snapshot)
    .bind(&request.stdout)
    .bind(&note)
    .bind(now)
    .execute(&state.db)
    .await?;

    get_submission_detail(&state, &submission_id, &user).await
}

pub(crate) async fn get_submission(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<SubmissionDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    get_submission_detail(&state, &id, &user).await
}

pub(crate) async fn review_submission(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<ReviewSubmissionRequest>,
) -> Result<Json<SubmissionDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let status = request.status.trim();
    if status != "reviewed" && status != "needs_work" {
        return Err(ApiError::bad_request(
            "status must be reviewed or needs_work",
        ));
    }
    let feedback = request
        .feedback
        .trim()
        .chars()
        .take(2000)
        .collect::<String>();
    let now = Utc::now().naive_utc();
    let updated = sqlx::query(
        "UPDATE submissions SET status = ?, parent_feedback = ?, reviewed_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(&feedback)
    .bind(now)
    .bind(&id)
    .execute(&state.db)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("submission not found"));
    }
    get_submission_detail(&state, &id, &user).await
}

async fn get_submission_detail(
    state: &AppState,
    id: &str,
    user: &crate::models::PublicUser,
) -> Result<Json<SubmissionDetail>, ApiError> {
    let row = if user.role == "parent" {
        sqlx::query_as::<_, SubmissionDetail>(
            "SELECT s.id, s.lesson_id, l.title AS lesson_title, u.display_name AS submitter_name,
                    s.status, s.note, s.code_snapshot, s.stdout, s.parent_feedback,
                    DATE_FORMAT(s.created_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS created_at,
                    DATE_FORMAT(s.reviewed_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS reviewed_at
             FROM submissions s
             JOIN lessons l ON l.id = s.lesson_id
             JOIN users u ON u.id = s.user_id
             WHERE s.id = ?",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, SubmissionDetail>(
            "SELECT s.id, s.lesson_id, l.title AS lesson_title, u.display_name AS submitter_name,
                    s.status, s.note, s.code_snapshot, s.stdout, s.parent_feedback,
                    DATE_FORMAT(s.created_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS created_at,
                    DATE_FORMAT(s.reviewed_at, '%Y-%m-%dT%H:%i:%s.%fZ') AS reviewed_at
             FROM submissions s
             JOIN lessons l ON l.id = s.lesson_id
             JOIN users u ON u.id = s.user_id
             WHERE s.id = ? AND s.user_id = ?",
        )
        .bind(id)
        .bind(&user.id)
        .fetch_optional(&state.db)
        .await?
    };
    row.map(Json)
        .ok_or_else(|| ApiError::not_found("submission not found"))
}
