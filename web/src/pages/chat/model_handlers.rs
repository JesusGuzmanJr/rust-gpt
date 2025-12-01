use {
    super::{types::ModelQuery, views::render_model_details},
    axum::{extract::Query, response::IntoResponse},
    tracing::*,
};

#[instrument]
pub(super) async fn get_models(
    Query(ModelQuery { model_id }): Query<ModelQuery>,
) -> impl IntoResponse {
    render_model_details(model_id.into()).into_response()
}
