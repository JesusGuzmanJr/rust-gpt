use {
    crate::{error::AppResult, persistence::db},
    anyhow::{Context, Result},
    argon2::Argon2,
    chrono::{DateTime, Utc},
    common::{key_type, string_type, uuid_type},
    garde::Validate,
    native_db::{Key, Models, ToKey, native_db},
    native_model::{Model, native_model},
    serde::{Deserialize, Serialize},
    tokio::task::spawn_blocking,
};

const SALT_LEN: usize = argon2::RECOMMENDED_SALT_LEN;
const HASH_LEN: usize = argon2::Params::DEFAULT_OUTPUT_LEN;

uuid_type!(
    /// A unique identifier for a user.
    pub UserId
);

string_type!(
    /// The name of a user. This can be a first name, last name, or a combination of both.
    #[derive(Validate)]
    pub(crate) Name(#[garde(length(min = 1, max = 64))])
);

string_type!(
    /// The email address of a user.
    #[derive(Validate)]
    pub(crate) EmailAddress(#[garde(email, length(max = 64))])
);

key_type!(
    /// The password of a user.
    #[derive(Validate)]
    #[garde(context(PasswordValidationContext))]
    pub(crate) Password(#[garde(length(min = 8, max = 64), custom(validate_entropy))])
);

/// Context for validating the entropy of a password.
#[derive(Debug)]
pub(crate) struct PasswordValidationContext {
    pub email: EmailAddress,
    pub name: Name,
}

fn validate_entropy(password: &str, context: &PasswordValidationContext) -> garde::Result {
    let user_inputs = [context.email.as_str(), context.name.as_str()];
    let entropy = zxcvbn::zxcvbn(password, &user_inputs);
    if entropy.score() < zxcvbn::Score::Three
    //  Can be cracked with 10^10 guesses or less.
    {
        if let Some(feedback) = entropy.feedback() {
            Err(garde::Error::new(format!(
                "The password entropy is too low: {}",
                feedback
            )))
        } else {
            Err(garde::Error::new("The password entropy is too low"))
        }
    } else {
        Ok(())
    }
}

impl ToKey for UserId {
    fn to_key(&self) -> Key {
        Key::new(self.0.as_bytes().to_vec())
    }

    fn key_names() -> Vec<String> {
        vec!["UserId".to_string()]
    }
}

impl ToKey for EmailAddress {
    fn to_key(&self) -> Key {
        Key::new(self.0.as_bytes().to_vec())
    }

    fn key_names() -> Vec<String> {
        vec!["EmailAddress".to_string()]
    }
}

pub(crate) type User = v1::User;
pub(crate) type UserKey = v1::UserKey;

pub(crate) mod v1 {
    use super::*;

    /// A user of the application.
    #[derive(Debug, Serialize, Deserialize)]
    #[native_model(id = 1, version = 1, with = native_model::bincode_2::Bincode)]
    #[native_db]
    pub(crate) struct User {
        #[primary_key]
        pub(crate) id: UserId,
        pub(crate) name: Name,
        #[secondary_key(unique)]
        pub(crate) email: EmailAddress,
        pub(super) hash: [u8; HASH_LEN],
        pub(super) salt: [u8; SALT_LEN],
        pub(crate) created_at: DateTime<Utc>,
        pub(crate) updated_at: DateTime<Utc>,
    }
}

pub(crate) fn define(models: &mut Models) -> Result<()> {
    models
        .define::<v1::User>()
        .context("failed to define user v1 model")
}

impl User {
    pub(crate) async fn new(name: Name, email: EmailAddress, password: Password) -> Result<Self> {
        spawn_blocking(move || {
            let mut salt = [0u8; SALT_LEN];
            rand::fill(&mut salt);

            let mut hash = [0u8; HASH_LEN];
            Argon2::default()
                .hash_password_into(password.0.as_bytes(), &salt, &mut hash)
                .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))?;

            Ok(Self {
                id: UserId::new(),
                name,
                email,
                hash,
                salt,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        })
        .await?
    }

    pub(crate) async fn save(self) -> Result<()> {
        spawn_blocking(move || {
            let rw = db()
                .rw_transaction()
                .context("failed to create rw transaction")?;

            rw.insert(self)?;
            rw.commit()
                .context("failed to commit transaction that saves user")?;

            Ok(())
        })
        .await?
    }

    pub(crate) async fn by_email(email: &EmailAddress) -> Result<Option<Self>> {
        let email = email.to_key();
        spawn_blocking(move || {
            db().r_transaction()?
                .get()
                .secondary(UserKey::email, email)
                .map_err(Into::into)
        })
        .await?
    }

    pub(crate) async fn by_id(id: UserId) -> Result<Option<Self>> {
        spawn_blocking(move || db().r_transaction()?.get().primary(id).map_err(Into::into)).await?
    }

    pub(crate) async fn verify_password(self, password: Password) -> Result<bool> {
        spawn_blocking(move || {
            let mut hash = [0u8; HASH_LEN];
            Argon2::default()
                .hash_password_into(password.0.as_bytes(), &self.salt, &mut hash)
                .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))?;

            Ok(hash == self.hash)
        })
        .await?
    }
}
