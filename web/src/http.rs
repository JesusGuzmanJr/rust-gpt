use axum::http::{HeaderName, HeaderValue, header};

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
