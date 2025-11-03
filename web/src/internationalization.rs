use {
    crate::{error::AppError, http::extract},
    axum::{extract::FromRequestParts, http::request::Parts},
    axum_extra::extract::CookieJar,
    chrono_tz::Tz,
    icu::locale::Locale,
};

// Extractor to get the user's locale and timezone from the cookies as set by
// the frontend script.
#[derive(Debug)]
pub(crate) struct Internationalization {
    pub(crate) locale: Locale,
    pub(crate) timezone: Tz,
}

impl FromRequestParts<()> for Internationalization {
    type Rejection = AppError;

    // Required method
    async fn from_request_parts(parts: &mut Parts, _: &()) -> Result<Self, Self::Rejection> {
        let cookie_jar = CookieJar::from_headers(&parts.headers);
        let locale = extract("locale", &cookie_jar).unwrap_or(icu::locale::locale!("en-US"));
        let timezone = extract("timezone", &cookie_jar).unwrap_or(chrono_tz::America::Los_Angeles);
        Ok(Internationalization { locale, timezone })
    }
}
