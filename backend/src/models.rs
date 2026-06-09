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
}

#[derive(Deserialize)]
pub(crate) struct CheckLessonRequest {
    pub(crate) code_snapshot: String,
    pub(crate) stdout: String,
}

#[derive(Serialize)]
pub(crate) struct CheckLessonResponse {
    pub(crate) passed: bool,
}

#[derive(FromRow, Serialize)]
pub(crate) struct LessonManageSummary {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) prompt: String,
    pub(crate) description: String,
    pub(crate) difficulty: String,
    pub(crate) is_published: bool,
    pub(crate) sort_order: i32,
}

#[derive(FromRow, Serialize)]
pub(crate) struct LessonManageDetail {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) prompt: String,
    pub(crate) description: String,
    pub(crate) hint: String,
    pub(crate) difficulty: String,
    pub(crate) starter_code: String,
    pub(crate) expected_stdout: String,
    pub(crate) hidden_tests: String,
    pub(crate) is_published: bool,
    pub(crate) sort_order: i32,
}

#[derive(Deserialize)]
pub(crate) struct CreateLessonRequest {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) prompt: String,
    pub(crate) description: String,
    pub(crate) hint: String,
    pub(crate) difficulty: Option<String>,
    pub(crate) starter_code: String,
    pub(crate) expected_stdout: String,
    pub(crate) hidden_tests: Option<String>,
    pub(crate) is_published: Option<bool>,
    pub(crate) sort_order: Option<i32>,
}

#[derive(Deserialize)]
pub(crate) struct UpdateLessonRequest {
    pub(crate) title: Option<String>,
    pub(crate) prompt: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) hint: Option<String>,
    pub(crate) difficulty: Option<String>,
    pub(crate) starter_code: Option<String>,
    pub(crate) expected_stdout: Option<String>,
    pub(crate) hidden_tests: Option<String>,
    pub(crate) is_published: Option<bool>,
    pub(crate) sort_order: Option<i32>,
}

#[derive(FromRow, Serialize)]
pub(crate) struct SubmissionSummary {
    pub(crate) id: String,
    pub(crate) lesson_id: String,
    pub(crate) lesson_title: String,
    pub(crate) submitter_name: String,
    pub(crate) status: String,
    pub(crate) note: String,
    pub(crate) parent_feedback: Option<String>,
    pub(crate) created_at: String,
    pub(crate) reviewed_at: Option<String>,
}

#[derive(FromRow, Serialize)]
pub(crate) struct SubmissionDetail {
    pub(crate) id: String,
    pub(crate) lesson_id: String,
    pub(crate) lesson_title: String,
    pub(crate) submitter_name: String,
    pub(crate) status: String,
    pub(crate) note: String,
    pub(crate) code_snapshot: String,
    pub(crate) stdout: String,
    pub(crate) parent_feedback: Option<String>,
    pub(crate) created_at: String,
    pub(crate) reviewed_at: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct CreateSubmissionRequest {
    pub(crate) lesson_id: String,
    pub(crate) code_snapshot: String,
    pub(crate) stdout: String,
    pub(crate) note: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ReviewSubmissionRequest {
    pub(crate) feedback: String,
    pub(crate) status: String,
}
