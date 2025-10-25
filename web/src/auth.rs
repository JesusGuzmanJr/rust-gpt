use {
    crate::{
        error::{AppError, AppResult},
        user::{User, UserId},
    },
    anyhow::{Context, Result},
    axum_extra::extract::{CookieJar, cookie::Cookie},
    base64::{
        Engine,
        engine::general_purpose::{GeneralPurpose, NO_PAD},
    },
    blake3::{Hash, KEY_LEN},
    chrono::{DateTime, Duration, Utc},
    common::{bincode, key_type},
    hex::FromHex,
    serde::{Deserialize, Deserializer, Serialize},
    std::sync::OnceLock,
};

const SESSION_DURATION: Duration = Duration::days(30);
const COOKIE_NAME: &str = "auth";

const BASE_64: GeneralPurpose = GeneralPurpose::new(&base64::alphabet::URL_SAFE, NO_PAD);

static KEY: OnceLock<[u8; KEY_LEN]> = OnceLock::new();

/// Initialize the Blake3 key.
#[deny(dead_code)]
pub(crate) fn init(config: AuthConfig) {
    KEY.set(config.blake3_key).expect("already initialized");
}

#[inline]
fn key() -> &'static [u8; KEY_LEN] {
    KEY.get().expect("not initialized")
}

key_type!(Blake3HexKey);

/// The SMTP server configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct AuthConfig {
    #[serde(deserialize_with = "deserialize_blake3_key")]
    blake3_key: [u8; KEY_LEN],
}

fn deserialize_blake3_key<'de, D>(deserializer: D) -> Result<[u8; KEY_LEN], D::Error>
where
    D: Deserializer<'de>,
{
    let hex_key = Blake3HexKey::deserialize(deserializer)?;

    let key = <[u8; KEY_LEN]>::from_hex(&hex_key.0)
        .context("failed to decode blake3_key")
        .map_err(serde::de::Error::custom)?;

    Ok(key)
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

    let cookie = BASE_64.encode(bincode::serialize(&Container {
        signature: *blake3::keyed_hash(
            key(),
            &bincode::serialize(&session).context("unable to serialize session")?,
        )
        .as_bytes(),
        session,
    })?);

    let mut cookie = Cookie::new(COOKIE_NAME, cookie);

    // cookie.set_secure(true);
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

    let Container { signature, session } = bincode::deserialize::<Container>(
        &BASE_64
            .decode(cookie.value())
            .context("unable to base64 decode cookie value")?,
    )?;

    if Hash::from(signature)
        != blake3::keyed_hash(
            key(),
            &bincode::serialize(&session).context("unable to serialize session")?,
        )
    {
        tracing::warn!("invalid auth signature");
        return Ok(None);
    }

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
