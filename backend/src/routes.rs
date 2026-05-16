use axum::{
    routing::{get, post, put},
    Json, Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    auth::{login, logout, me},
    error::ApiError,
    lessons::{check_lesson, get_lesson, list_lessons},
    models::HealthResponse,
    projects::{create_project, delete_project, get_project, list_projects, save_project_files},
    state::AppState,
};

pub(crate) fn build_router(state: AppState) -> Router {
    Router::new()
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
        .with_state(state)
}

async fn health() -> Result<Json<HealthResponse>, ApiError> {
    Ok(Json(HealthResponse { ok: true }))
}
