use axum::{
    routing::{get, post, put},
    Json, Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    auth::{login, logout, me},
    error::ApiError,
    lessons::{
        check_lesson, create_lesson, get_lesson, get_lesson_manage, list_lessons,
        list_lessons_manage, update_lesson,
    },
    models::HealthResponse,
    projects::{create_project, delete_project, get_project, list_projects, save_project_files},
    state::AppState,
    submissions::{create_submission, get_submission, list_submissions, review_submission},
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
        .route("/api/lessons", get(list_lessons).post(create_lesson))
        .route("/api/lessons/manage", get(list_lessons_manage))
        .route("/api/lessons/:id", get(get_lesson).put(update_lesson))
        .route("/api/lessons/:id/manage", get(get_lesson_manage))
        .route("/api/lessons/:id/check", post(check_lesson))
        .route(
            "/api/submissions",
            get(list_submissions).post(create_submission),
        )
        .route("/api/submissions/:id", get(get_submission))
        .route("/api/submissions/:id/review", put(review_submission))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> Result<Json<HealthResponse>, ApiError> {
    Ok(Json(HealthResponse { ok: true }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{header, Method, Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tempfile::TempDir;
    use tower::ServiceExt;

    use super::*;
    use crate::{
        db::{seed_lessons, seed_users},
        state::AppState,
        test_db::TestDb,
    };

    async fn test_state() -> anyhow::Result<Option<(AppState, TempDir, TestDb)>> {
        let Some(db) = TestDb::connect().await? else {
            return Ok(None);
        };
        let temp_dir = tempfile::tempdir()?;
        seed_users(&db.pool).await?;
        seed_lessons(&db.pool).await?;
        Ok(Some((
            AppState {
                db: db.pool.clone(),
                data_dir: temp_dir.path().to_path_buf(),
            },
            temp_dir,
            db,
        )))
    }

    async fn json_body(response: axum::response::Response) -> anyhow::Result<Value> {
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn login_cookie(app: Router, username: &str, password: &str) -> anyhow::Result<String> {
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "username": username, "password": password }).to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        Ok(response
            .headers()
            .get(header::SET_COOKIE)
            .expect("session cookie")
            .to_str()?
            .to_string())
    }

    #[tokio::test]
    async fn login_returns_child_role_and_rejects_bad_password() -> anyhow::Result<()> {
        let Some((state, _temp_dir, _db)) = test_state().await? else {
            return Ok(());
        };
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "username": "son", "password": "python" }).to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await?;
        assert_eq!(body["user"]["role"], "child");
        assert_eq!(body["user"]["display_name"], "Young Coder");

        let bad = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "username": "son", "password": "wrong" }).to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn lesson_routes_return_rich_lessons_and_record_attempts() -> anyhow::Result<()> {
        let Some((state, _temp_dir, _db)) = test_state().await? else {
            return Ok(());
        };
        let app = build_router(state.clone());
        let cookie = login_cookie(app.clone(), "son", "python").await?;

        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/lessons")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(list_response.status(), StatusCode::OK);
        let lessons = json_body(list_response).await?;
        assert_eq!(lessons.as_array().expect("lesson array").len(), 26);
        assert_eq!(lessons[0]["difficulty"], "Basics");
        assert!(lessons[0]["description"]
            .as_str()
            .unwrap()
            .contains("print"));

        let detail_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/lessons/hello-python")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(detail_response.status(), StatusCode::OK);
        let detail = json_body(detail_response).await?;
        assert_eq!(
            detail["hint"],
            "The text inside print() needs quotation marks."
        );
        assert!(detail.get("expected_stdout").is_none());

        let check_response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/lessons/hello-python/check")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "code_snapshot": "print(\"hello, python\")\n",
                            "stdout": "hello, python\n"
                        })
                        .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(check_response.status(), StatusCode::OK);
        let check = json_body(check_response).await?;
        assert_eq!(check["passed"], true);
        assert!(check.get("expected_stdout").is_none());

        let attempts = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM attempts")
            .fetch_one(&state.db)
            .await?;
        assert_eq!(attempts, 1);
        Ok(())
    }

    #[tokio::test]
    async fn unpublished_lessons_cannot_be_checked() -> anyhow::Result<()> {
        let Some((state, _temp_dir, _db)) = test_state().await? else {
            return Ok(());
        };
        sqlx::query("UPDATE lessons SET is_published = 0 WHERE id = ?")
            .bind("hello-python")
            .execute(&state.db)
            .await?;
        let app = build_router(state.clone());
        let cookie = login_cookie(app.clone(), "son", "python").await?;

        let detail_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/lessons/hello-python")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(detail_response.status(), StatusCode::NOT_FOUND);

        let check_response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/lessons/hello-python/check")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "code_snapshot": "print(\"hello, python\")\n",
                            "stdout": "hello, python\n"
                        })
                        .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(check_response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn lesson_management_routes_require_parent_and_update_lessons() -> anyhow::Result<()> {
        let Some((state, _temp_dir, _db)) = test_state().await? else {
            return Ok(());
        };
        let app = build_router(state);
        let parent_cookie = login_cookie(app.clone(), "parent", "change-me").await?;
        let child_cookie = login_cookie(app.clone(), "son", "python").await?;

        let child_manage_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/lessons/manage")
                    .header(header::COOKIE, &child_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(child_manage_response.status(), StatusCode::FORBIDDEN);

        let child_create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/lessons")
                    .header(header::COOKIE, &child_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "id": "child-lesson",
                            "title": "Child Lesson",
                            "prompt": "Print from child.",
                            "description": "Should be forbidden.",
                            "hint": "",
                            "starter_code": "print(\"nope\")\n",
                            "expected_stdout": "nope\n"
                        })
                        .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(child_create_response.status(), StatusCode::FORBIDDEN);

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/lessons")
                    .header(header::COOKIE, &parent_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "id": "new-lesson",
                            "title": "New Lesson",
                            "prompt": "Print something new.",
                            "description": "Practice output.",
                            "hint": "Use print().",
                            "difficulty": "Basics",
                            "starter_code": "print(\"new\")\n",
                            "expected_stdout": "new\n",
                            "hidden_tests": "[]",
                            "is_published": true,
                            "sort_order": 99
                        })
                        .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(create_response.status(), StatusCode::OK);
        let created = json_body(create_response).await?;
        assert_eq!(created["id"], "new-lesson");
        assert_eq!(created["expected_stdout"], "new\n");

        let manage_detail_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/lessons/new-lesson/manage")
                    .header(header::COOKIE, &parent_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(manage_detail_response.status(), StatusCode::OK);
        assert_eq!(
            json_body(manage_detail_response).await?["hidden_tests"],
            "[]"
        );

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/lessons/new-lesson")
                    .header(header::COOKIE, &parent_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "title": "Updated Lesson",
                            "is_published": false
                        })
                        .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(update_response.status(), StatusCode::OK);
        let updated = json_body(update_response).await?;
        assert_eq!(updated["title"], "Updated Lesson");
        assert_eq!(updated["is_published"], false);

        let child_detail_response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/lessons/new-lesson")
                    .header(header::COOKIE, &child_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(child_detail_response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn submission_routes_allow_child_submit_and_parent_review() -> anyhow::Result<()> {
        let Some((state, _temp_dir, _db)) = test_state().await? else {
            return Ok(());
        };
        let app = build_router(state);
        let parent_cookie = login_cookie(app.clone(), "parent", "change-me").await?;
        let child_cookie = login_cookie(app.clone(), "son", "python").await?;

        let parent_create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/submissions")
                    .header(header::COOKIE, &parent_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "lesson_id": "hello-python",
                            "code_snapshot": "print(\"parent\")\n",
                            "stdout": "parent\n"
                        })
                        .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(parent_create_response.status(), StatusCode::FORBIDDEN);

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/submissions")
                    .header(header::COOKIE, &child_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "lesson_id": "hello-python",
                            "code_snapshot": "print(\"hello, python\")\n",
                            "stdout": "hello, python\n",
                            "note": "Ready for review"
                        })
                        .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(create_response.status(), StatusCode::OK);
        let created = json_body(create_response).await?;
        let submission_id = created["id"].as_str().expect("submission id");
        assert_eq!(created["status"], "pending");

        let parent_list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/submissions")
                    .header(header::COOKIE, &parent_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(parent_list_response.status(), StatusCode::OK);
        let parent_list = json_body(parent_list_response).await?;
        assert_eq!(parent_list.as_array().expect("submission array").len(), 1);
        assert_eq!(parent_list[0]["parent_feedback"], Value::Null);

        let child_review_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/api/submissions/{submission_id}/review"))
                    .header(header::COOKIE, &child_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "feedback": "Looks good", "status": "reviewed" }).to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(child_review_response.status(), StatusCode::FORBIDDEN);

        let review_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/api/submissions/{submission_id}/review"))
                    .header(header::COOKIE, &parent_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "feedback": "Nice work", "status": "reviewed" }).to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(review_response.status(), StatusCode::OK);
        let reviewed = json_body(review_response).await?;
        assert_eq!(reviewed["status"], "reviewed");
        assert_eq!(reviewed["parent_feedback"], "Nice work");

        let child_get_response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/submissions/{submission_id}"))
                    .header(header::COOKIE, &child_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(child_get_response.status(), StatusCode::OK);
        assert_eq!(
            json_body(child_get_response).await?["parent_feedback"],
            "Nice work"
        );
        Ok(())
    }

    #[tokio::test]
    async fn project_routes_create_update_get_and_delete_owned_projects() -> anyhow::Result<()> {
        let Some((state, _temp_dir, _db)) = test_state().await? else {
            return Ok(());
        };
        let app = build_router(state);
        let cookie = login_cookie(app.clone(), "parent", "change-me").await?;

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/projects")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "title": "Practice", "starter_code": "print(1)\n" }).to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(create_response.status(), StatusCode::OK);
        let created = json_body(create_response).await?;
        let project_id = created["id"].as_str().expect("project id");

        let save_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/api/projects/{project_id}/files"))
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "files": { "main.py": "print(2)\n" } }).to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(save_response.status(), StatusCode::OK);
        assert_eq!(
            json_body(save_response).await?["files"]["main.py"],
            "print(2)\n"
        );

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/projects/{project_id}"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(get_response.status(), StatusCode::OK);
        assert_eq!(json_body(get_response).await?["title"], "Practice");

        let delete_response = app
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/projects/{project_id}"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
        Ok(())
    }

    #[tokio::test]
    async fn project_routes_require_parent_role() -> anyhow::Result<()> {
        let Some((state, _temp_dir, _db)) = test_state().await? else {
            return Ok(());
        };
        let app = build_router(state);
        let child_cookie = login_cookie(app.clone(), "son", "python").await?;

        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/projects")
                    .header(header::COOKIE, &child_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(list_response.status(), StatusCode::FORBIDDEN);

        let create_response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/projects")
                    .header(header::COOKIE, &child_cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "title": "Child Project", "starter_code": "print(1)\n" })
                            .to_string(),
                    ))?,
            )
            .await?;
        assert_eq!(create_response.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
