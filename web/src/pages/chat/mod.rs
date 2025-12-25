mod auth_handlers;
mod message_handlers;
mod model_handlers;
mod sse;
mod thread_handlers;
mod types;
mod views;

use {
    auth_handlers::sign_out,
    axum::{
        Router,
        routing::{get, post},
    },
    message_handlers::{send_message, stream_response, update_feedback, update_message},
    model_handlers::get_models,
    thread_handlers::{delete_thread, new_thread, select_thread, update_title},
};

pub(crate) const PATH: &str = "/chat";

pub(crate) use views::page;

pub(crate) fn api() -> Router {
    Router::new().nest(
        PATH,
        Router::new()
            .route("/models", get(get_models))
            .route("/send", post(send_message))
            .route("/title", post(update_title))
            .route("/sign-out", get(sign_out))
            .route("/feedback", post(update_feedback))
            .route("/update", post(update_message))
            .route("/new", post(new_thread))
            .route("/select", get(select_thread))
            .route("/delete", post(delete_thread))
            .route("/response", get(stream_response)),
    )
}
