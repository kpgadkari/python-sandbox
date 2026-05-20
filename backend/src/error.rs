use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ApiError {
    #[error("{message}")]
    Http { status: StatusCode, message: String },
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl ApiError {
    pub(crate) fn unauthorized() -> Self {
        Self::Http {
            status: StatusCode::UNAUTHORIZED,
            message: "unauthorized".into(),
        }
    }

    pub(crate) fn forbidden(message: &str) -> Self {
        Self::Http {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }

    pub(crate) fn not_found(message: &str) -> Self {
        Self::Http {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub(crate) fn bad_request(message: &str) -> Self {
        Self::Http {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub(crate) fn internal(message: &str) -> Self {
        Self::Http {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Http { status, message } => {
                (status, Json(ErrorResponse { error: message })).into_response()
            }
            ApiError::Database(err) => {
                tracing::error!("database error: {err}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "database error".into(),
                    }),
                )
                    .into_response()
            }
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}
