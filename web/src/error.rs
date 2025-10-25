use {axum::http::StatusCode, thiserror::Error};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("😭 An unexpected error occurred: {0}")]
    InternalServerError(#[from] anyhow::Error),

    #[error("Unable to create user because email already in use")]
    DuplicateEmail,
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use AppError::*;
        let status = match self {
            InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DuplicateEmail => StatusCode::BAD_REQUEST,
        };
        (status, self.to_string()).into_response()
    }
}
