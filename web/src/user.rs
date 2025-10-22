use {
    common::{key_type, string_type, uuid_type},
    garde::Validate,
};

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
