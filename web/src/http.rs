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

/// Helper to get a T from the given cookie name.
fn extract<T>(cookie_name: &str, cookie_jar: &CookieJar) -> Option<T>
where
    T: std::str::FromStr,
{
    cookie_jar
        .get(cookie_name)
        .map(|cookie| cookie.value())
        .and_then(|value| value.parse::<T>().ok())
}

/// Helper to get the user's locale from the cookies set by the
/// frontend script.
pub(crate) fn extract_locale(cookie_jar: &CookieJar) -> Locale {
    extract("locale", cookie_jar).unwrap_or(icu::locale::locale!("en-US"))
}

/// Helper to get the user's timezone from the cookies set by the
/// frontend script (chrono-tz format).
pub(crate) fn extract_timezone(cookie_jar: &CookieJar) -> Tz {
    extract("timezone", cookie_jar).unwrap_or(chrono_tz::America::Los_Angeles)
}

/// Helper to get the current thread ID from the cookies.
pub(crate) fn extract_thread_id(cookie_jar: &CookieJar) -> Option<ThreadId> {
    extract("thread_id", cookie_jar)
}

/// Helper to get the current user ID from the cookies.
pub(crate) fn extract_user_id(cookie_jar: &CookieJar) -> Option<UserId> {
    extract("user_id", cookie_jar)
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
