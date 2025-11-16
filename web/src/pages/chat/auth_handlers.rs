use {
    axum::response::{IntoResponse, Redirect},
    axum_extra::extract::CookieJar,
    tracing::*,
};

#[instrument]
pub(super) async fn sign_out(cookie_jar: CookieJar) -> impl IntoResponse {
    (
        crate::auth::remove_auth_cookie(cookie_jar),
        Redirect::to(crate::pages::signin::PATH),
    )
}
