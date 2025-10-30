use {
    crate::{thread::ThreadId, user::UserId},
    axum::http::{HeaderName, HeaderValue, header},
    axum_extra::extract::CookieJar,
    chrono_tz::Tz,
    icu::locale::Locale,
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

/// Helper to get the user's locale from the cookies set by the
/// frontend script.
pub(crate) fn extract_locale(cookie_jar: &CookieJar) -> Locale {
    cookie_jar
        .get("locale")
        .map(|cookie| cookie.value())
        .unwrap_or("en-US")
        .parse()
        .unwrap_or(icu::locale::locale!("en-US"))
}

/// Helper to get the user's timezone from the cookies set by the
/// frontend script (chrono-tz format).
pub(crate) fn extract_timezone(cookie_jar: &CookieJar) -> Tz {
    cookie_jar
        .get("timezone")
        .and_then(|cookie| cookie.value().parse().ok())
        .unwrap_or(chrono_tz::America::Los_Angeles)
}

/// Helper to get the current thread ID from the cookies.
pub(crate) fn extract_thread_id(cookie_jar: &CookieJar) -> Option<ThreadId> {
    cookie_jar
        .get("chat_id")
        .map(|cookie| cookie.value())
        .and_then(|value| ThreadId::try_parse(value).ok())
}

/// Helper to get the current user ID from the cookies.
pub(crate) fn extract_user_id(cookie_jar: &CookieJar) -> Option<UserId> {
    cookie_jar
        .get("user_id")
        .map(|cookie| cookie.value())
        .and_then(|value| UserId::try_parse(value).ok())
}

#[cfg(test)]
mod tests {
    use {super::*, axum_extra::extract::cookie::Cookie};

    #[test]
    fn test_default_locale() {
        let cookie_jar = CookieJar::new();
        let locale = extract_locale(&cookie_jar);
        assert_eq!(locale, icu::locale::locale!("en-US"));
    }

    #[test]
    fn test_default_timezone() {
        let cookie_jar = CookieJar::new();
        let timezone = extract_timezone(&cookie_jar);
        assert_eq!(timezone, chrono_tz::America::Los_Angeles);
    }

    #[test]
    fn test_custom_locale() {
        let cookie_jar = CookieJar::new().add(Cookie::new("locale", "es-ES"));
        let locale = extract_locale(&cookie_jar);
        assert_eq!(locale, icu::locale::locale!("es-ES"));
    }

    #[test]
    fn test_custom_timezone() {
        let cookie_jar = CookieJar::new().add(Cookie::new("timezone", "America/New_York"));
        let timezone = extract_timezone(&cookie_jar);
        assert_eq!(timezone, chrono_tz::America::New_York);
    }
}
