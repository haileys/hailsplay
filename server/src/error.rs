use axum::{response::{Response, IntoResponse}, http::StatusCode};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        log::error!("http request error: {:?}\n{}", self.0, self.0.backtrace());

        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self.0)).into_response()
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
