use {
    axum::http::{HeaderName, HeaderValue, header},
    axum_extra::extract::CookieJar,
};

/// 15 minutes public cache
pub(crate) const CACHE_PUBLICLY_15_MIN: (HeaderName, HeaderValue) = (
    header::CACHE_CONTROL,
    HeaderValue::from_static("public, max-age=900"), // 15 minutes
);

/// HTML content type
pub(crate) const HTML_CONTENT_TYPE: (HeaderName, HeaderValue) = (
    header::CONTENT_TYPE,
    HeaderValue::from_static("text/html; charset=utf-8"),
);

/// Helper to get a T from the given cookie name.
pub(crate) fn extract<T>(cookie_name: &str, cookie_jar: &CookieJar) -> Option<T>
where
    T: std::str::FromStr,
{
    cookie_jar
        .get(cookie_name)
        .map(|cookie| cookie.value())
        .and_then(|value| value.parse::<T>().ok())
}
