use std::{collections::HashMap, fs};

use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    auth::require_user,
    error::ApiError,
    models::{CreateProjectRequest, ProjectDetail, ProjectSummary, PublicUser, SaveFilesRequest},
    project_files::{read_project_files, validate_files, write_project_files},
    state::AppState,
};

pub(crate) async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProjectSummary>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let rows = sqlx::query_as::<_, ProjectSummary>(
        "SELECT id, title, created_at, updated_at FROM projects WHERE owner_id = ? ORDER BY updated_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub(crate) async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> Result<Json<ProjectDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
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

    sqlx::query(
        "INSERT INTO projects (id, owner_id, title, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&project_id)
    .bind(&user.id)
    .bind(&title)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(ProjectDetail {
        id: project_id,
        title,
        created_at: now.clone(),
        updated_at: now,
        files,
    }))
}

pub(crate) async fn get_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ProjectDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let summary = project_for_owner(&state, &id, &user.id).await?;
    let files = read_project_files(&state.data_dir, &id)?;
    Ok(Json(ProjectDetail {
        id: summary.id,
        title: summary.title,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
        files,
    }))
}

pub(crate) async fn save_project_files(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<SaveFilesRequest>,
) -> Result<Json<ProjectDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    let mut summary = project_for_owner(&state, &id, &user.id).await?;
    validate_files(&request.files)?;
    write_project_files(&state.data_dir, &id, &request.files)?;

    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE projects SET updated_at = ? WHERE id = ? AND owner_id = ?")
        .bind(&now)
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;
    summary.updated_at = now;

    Ok(Json(ProjectDetail {
        id: summary.id,
        title: summary.title,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
        files: request.files,
    }))
}

pub(crate) async fn delete_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let user = require_user(&state, &headers).await?;
    require_parent(&user)?;
    project_for_owner(&state, &id, &user.id).await?;
    sqlx::query("DELETE FROM projects WHERE id = ? AND owner_id = ?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;
    let project_dir = state.data_dir.join("projects").join(&id);
    let _ = fs::remove_dir_all(project_dir);
    Ok(StatusCode::NO_CONTENT)
}

async fn project_for_owner(
    state: &AppState,
    project_id: &str,
    owner_id: &str,
) -> Result<ProjectSummary, ApiError> {
    sqlx::query_as::<_, ProjectSummary>(
        "SELECT id, title, created_at, updated_at FROM projects WHERE id = ? AND owner_id = ?",
    )
    .bind(project_id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::not_found("project not found"))
}

fn require_parent(user: &PublicUser) -> Result<(), ApiError> {
    if user.role == "parent" {
        Ok(())
    } else {
        Err(ApiError::forbidden("projects require parent access"))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_titles() {
        assert_eq!(clean_title(None), "Untitled Project");
        assert_eq!(
            clean_title(Some("  My Project  ".to_string())),
            "My Project"
        );
        assert_eq!(clean_title(Some("   ".to_string())), "Untitled Project");
        assert_eq!(clean_title(Some("x".repeat(100))).chars().count(), 80);
    }
}
