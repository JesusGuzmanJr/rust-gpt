use {
    crate::user::UserId,
    anyhow::Result,
    axum_extra::extract::{CookieJar, cookie::Cookie},
    base64::{
        Engine,
        engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig},
    },
    blake3::KEY_LEN,
    chrono::{DateTime, Duration, Utc},
    common::bincode,
    serde::{Deserialize, Serialize},
    std::sync::OnceLock,
};

const SESSION_DURATION: Duration = Duration::days(30);
const COOKIE_NAME: &str = "auth";

const BASE_64: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    GeneralPurposeConfig::new().with_encode_padding(false),
);

static KEY: OnceLock<[u8; KEY_LEN]> = OnceLock::new();

/// Initialize the Blake3 key.
#[deny(dead_code)]
pub(crate) fn init(key: [u8; KEY_LEN]) {
    KEY.set(key).expect("already initialized");
}

#[inline]
fn key() -> &'static [u8; KEY_LEN] {
    KEY.get().expect("not initialized")
}

#[derive(Serialize, Deserialize)]
struct Session {
    user_id: UserId,
    create_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct Container {
    session: Session,
    signature: [u8; blake3::OUT_LEN],
}

pub(crate) fn create_auth_cookie(cookie_jar: CookieJar, user_id: UserId) -> Result<CookieJar> {
    let session = Session {
        user_id,
        create_at: Utc::now(),
    };

    let mut cookie = Cookie::new(
        COOKIE_NAME,
        BASE_64.encode(bincode::serialize(&Container {
            signature: *blake3::keyed_hash(key(), &bincode::serialize(&session)?).as_bytes(),
            session,
        })?),
    );

    cookie.set_secure(true);
    cookie.set_same_site(axum_extra::extract::cookie::SameSite::Strict);
    cookie.set_http_only(true);
    cookie.set_max_age(time::Duration::seconds(SESSION_DURATION.num_seconds()));

    Ok(cookie_jar.add(cookie))
}

pub(crate) fn remove_auth_cookie(cookie_jar: CookieJar) -> Result<CookieJar> {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_max_age(Some(time::Duration::seconds(0)));
    Ok(cookie_jar.add(cookie))
}
