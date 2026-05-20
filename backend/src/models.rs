use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    pub(crate) ok: bool,
}

#[derive(Deserialize)]
pub(crate) struct LoginRequest {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Serialize)]
pub(crate) struct MeResponse {
    pub(crate) user: PublicUser,
}

#[derive(Clone, FromRow, Serialize)]
pub(crate) struct PublicUser {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
}

#[derive(FromRow)]
pub(crate) struct UserWithHash {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) password_hash: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
}

impl UserWithHash {
    pub(crate) fn into_public(self) -> PublicUser {
        PublicUser {
            id: self.id,
            username: self.username,
            display_name: self.display_name,
            role: self.role,
        }
    }
}

#[derive(FromRow, Serialize)]
pub(crate) struct ProjectSummary {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct ProjectDetail {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) files: HashMap<String, String>,
}

#[derive(Deserialize)]
pub(crate) struct CreateProjectRequest {
    pub(crate) title: Option<String>,
    pub(crate) starter_code: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct SaveFilesRequest {
    pub(crate) files: HashMap<String, String>,
}

#[derive(FromRow, Serialize)]
pub(crate) struct LessonSummary {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) prompt: String,
    pub(crate) description: String,
    pub(crate) difficulty: String,
}

#[derive(FromRow, Serialize)]
pub(crate) struct LessonDetail {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) prompt: String,
    pub(crate) description: String,
    pub(crate) hint: String,
    pub(crate) difficulty: String,
    pub(crate) starter_code: String,
    pub(crate) expected_stdout: String,
}

#[derive(Deserialize)]
pub(crate) struct CheckLessonRequest {
    pub(crate) code_snapshot: String,
    pub(crate) stdout: String,
}

#[derive(Serialize)]
pub(crate) struct CheckLessonResponse {
    pub(crate) passed: bool,
    pub(crate) expected_stdout: String,
}
