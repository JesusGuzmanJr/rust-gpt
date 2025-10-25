use {
    axum::{
        http::StatusCode,
        response::{Redirect, Response},
    },
    thiserror::Error,
};

pub(crate) type AppResult<T> = Result<T, AppError>;
pub(crate) type ResponseResult = Result<Response, AppError>;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("😭 An unexpected error occurred: {0}")]
    InternalServerError(#[from] anyhow::Error),

    #[error("❌ You're not logged in")]
    Unauthorized,

    #[error("⚠️ Unable to create user account because the email is already in use")]
    DuplicateEmail,
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> Response {
        use AppError::*;
        let status = match self {
            InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Unauthorized => StatusCode::UNAUTHORIZED,
            DuplicateEmail => StatusCode::BAD_REQUEST,
        };

        if matches!(self, Unauthorized) {
            return Redirect::to(crate::pages::signin::PATH).into_response();
        }

        (status, self.to_string()).into_response()
    }
}
