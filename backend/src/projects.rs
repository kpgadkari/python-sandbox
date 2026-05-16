use std::{collections::HashMap, fs};

use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::{
    auth::require_user,
    error::ApiError,
    models::{CreateProjectRequest, ProjectDetail, ProjectSummary, SaveFilesRequest},
    project_files::{read_project_files, validate_files, write_project_files},
    state::AppState,
};

pub(crate) async fn list_projects(
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

pub(crate) async fn create_project(
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

pub(crate) async fn get_project(
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

pub(crate) async fn save_project_files(
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

pub(crate) async fn delete_project(
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
