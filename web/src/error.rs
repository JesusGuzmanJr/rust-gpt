use {
    axum::{
        http::StatusCode,
        response::{IntoResponse, Redirect, Response},
    },
    std::any::Any,
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
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use AppError::*;
        let status = match self {
            InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Unauthorized => StatusCode::UNAUTHORIZED,
        };

        if matches!(self, Unauthorized) {
            return Redirect::to(crate::pages::signin::PATH).into_response();
        }

        (status, self.to_string()).into_response()
    }
}

#[derive(Copy, Clone)]
pub(crate) struct PanicResponder;

impl tower_http::catch_panic::ResponseForPanic for PanicResponder {
    type ResponseBody = axum::body::Body;

    fn response_for_panic(
        &mut self,
        panic: Box<dyn Any + Send + 'static>,
    ) -> Response<Self::ResponseBody> {
        tracing::error!(?panic);
        crate::pages::internal_server_error_page().into_response()
    }
}
