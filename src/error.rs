use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::render::render_error_response;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound(&'static str),
    UnprocessableEntity(&'static str),
    Internal(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(message) => {
                render_error_response(StatusCode::BAD_REQUEST, "400", &message)
            }
            Self::NotFound(message) => render_error_response(StatusCode::NOT_FOUND, "404", message),
            Self::UnprocessableEntity(message) => {
                render_error_response(StatusCode::UNPROCESSABLE_ENTITY, "422", message)
            }
            Self::Internal(error) => {
                error!("request failed: {error}");
                render_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "500",
                    "Internal server error.",
                )
            }
        }
    }
}

pub fn bad_request<E>(error: E) -> AppError
where
    E: std::fmt::Display,
{
    AppError::BadRequest(error.to_string())
}
