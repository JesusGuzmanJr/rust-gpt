use {
    anyhow::{Context, Result},
    argon2::Argon2,
    chrono::{DateTime, Utc},
    common::{key_type, string_type, uuid_type},
    garde::Validate,
    native_db::{Key, Models, ToKey, native_db},
    native_model::{Model, native_model},
    serde::{Deserialize, Serialize},
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

pub(crate) type User = v1::User;

mod v1 {
    use super::*;

    /// A user of the application.
    #[derive(Debug, Serialize, Deserialize)]
    #[native_model(id = 1, version = 1)]
    #[native_db]
    pub(super) struct User {
        #[primary_key]
        pub(super) id: UserId,
        pub(super) name: Name,
        pub(super) email: EmailAddress,
        pub(super) hash: [u8; HASH_LEN],
        pub(super) salt: [u8; SALT_LEN],
        pub(super) created_at: DateTime<Utc>,
        pub(super) updated_at: DateTime<Utc>,
    }
}

pub(crate) fn define(models: &mut Models) -> Result<()> {
    models
        .define::<v1::User>()
        .context("failed to define user v1 model")
}

impl User {
    pub(crate) fn new(name: Name, email: EmailAddress, password: Password) -> Result<Self> {
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
    }

    pub(crate) fn verify_password(&self, password: &Password) -> Result<bool> {
        let mut hash = [0u8; HASH_LEN];
        Argon2::default()
            .hash_password_into(password.0.as_bytes(), &self.salt, &mut hash)
            .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))?;

        Ok(hash == self.hash)
    }
}
