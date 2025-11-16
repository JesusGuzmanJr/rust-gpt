use {
    super::{types::ModelQuery, views::render_model_details},
    axum::{extract::Query, response::IntoResponse},
    tracing::*,
};

#[instrument]
pub(super) async fn get_models(Query(ModelQuery { model }): Query<ModelQuery>) -> impl IntoResponse {
    render_model_details(model).into_response()
}
