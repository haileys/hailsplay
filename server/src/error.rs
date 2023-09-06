use axum::response::{Response, IntoResponse};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct AppError(anyhow::Error);

#[derive(Serialize)]
struct ErrorInfo {
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        log::error!("http request error: {:?}\n{}", self.0, self.0.backtrace());

        let error = ErrorInfo {
            message: format!("{:?}", self.0),
        };

        (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
