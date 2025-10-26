use {
    crate::{
        error::{AppError, AppResult},
        hash::Container,
        user::{User, UserId},
    },
    anyhow::Result,
    axum_extra::extract::{CookieJar, cookie::Cookie},
    chrono::{DateTime, Duration, Utc},
    serde::{Deserialize, Serialize},
    std::str::FromStr,
};

const SESSION_DURATION: Duration = Duration::days(30);
const COOKIE_NAME: &str = "auth";

#[derive(Serialize, Deserialize)]
struct Session {
    user_id: UserId,
    create_at: DateTime<Utc>,
}

pub(crate) fn create_auth_cookie(cookie_jar: CookieJar, user_id: UserId) -> Result<CookieJar> {
    let session = Session {
        user_id,
        create_at: Utc::now(),
    };

    let mut cookie = Cookie::new(COOKIE_NAME, Container::new(session)?.to_string());

    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Strict);
    cookie.set_http_only(true);
    cookie.set_max_age(time::Duration::seconds(SESSION_DURATION.num_seconds()));
    cookie.set_secure(!cfg!(debug_assertions));
    cookie.set_path("/");

    Ok(cookie_jar.add(cookie))
}

pub(crate) fn remove_auth_cookie(cookie_jar: CookieJar) -> CookieJar {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Strict);
    cookie.set_path("/");
    cookie.set_max_age(Some(time::Duration::seconds(0)));
    cookie_jar.add(cookie)
}

fn extract_session(cookie_jar: &CookieJar) -> Result<Option<Session>> {
    let cookie = match cookie_jar.get(COOKIE_NAME) {
        None => return Ok(None),
        Some(cookie) => cookie,
    };

    let session = Container::<Session>::from_str(cookie.value())?.into_inner();

    if Utc::now() - session.create_at > SESSION_DURATION {
        tracing::warn!("auth session expired");
        return Ok(None);
    }

    Ok(Some(session))
}

pub(crate) fn require_auth_user(cookie_jar: &CookieJar) -> AppResult<User> {
    let session = extract_session(cookie_jar)?.ok_or(AppError::Unauthorized)?;
    let user = User::by_id(session.user_id)?.ok_or(AppError::Unauthorized)?;
    Ok(user)
}
