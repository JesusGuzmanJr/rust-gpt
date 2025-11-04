use {
    crate::{
        TEAM_EMAIL,
        error::AppResult,
        hash::GlassVault,
        mailer, svg,
        user::{EmailAddress, Name, Password, User},
    },
    axum::{
        Form, Router,
        extract::{ConnectInfo, Query},
        response::IntoResponse,
        routing::{get, post},
    },
    axum_valid::Garde,
    chrono::{DateTime, Duration, Utc},
    garde::{Validate, util::nested_path},
    maud::{Markup, html},
    serde::{Deserialize, Serialize},
    std::{net::SocketAddr, str::FromStr},
    tracing::*,
};

pub(crate) const PATH: &str = "/signup";

const VERIFICATION_LINK_EXPIRATION: Duration = Duration::hours(24);

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "Sign Up",
        html! {
            div.auth-container {
                div.auth-card {
                    (signup_card(false))
                }
            }
        },
    )
}

fn signup_card(email_is_taken: bool) -> Markup {
    html! {
        div.auth-card__header {
            div.auth-logo {
                span.auth-logo__text { "AI" }
            }
            h1.auth-title { "Create Account" }
            p.auth-subtitle { "Sign up to get started with " (crate::PROJECT_NAME) }
        }

        form.auth-form hx-post="/api/signup" hx-target=".auth-card" autocomplete="on" {
            @if email_is_taken {
                div.auth-error {
                    (svg::x_circle("auth-error__icon", 20, 20))
                    span.auth-error__text { "This email address is already in use. Please sign in instead." }
                }
            }

            div.form-group {
                label.form-label for="name" { "Full Name" }
                div.input-wrapper {
                    (svg::user("input-icon", 20, 20))
                    input."form-input"#name type="text" name="name" placeholder="Jane Doe" autocomplete="name" required autofocus;
                }
            }

            div.form-group {
                label.form-label for="email" { "Email" }
                div.input-wrapper {
                    (svg::envelope("input-icon", 20, 20))
                    input."form-input"#email
                        type="email"
                        name="email"
                        autocomplete="username"
                        inputmode="email"
                        placeholder="you@example.com"
                        required
                        hx-get="/api/signup/validate"
                        hx-trigger="keyup changed delay:500ms"
                        hx-target="#email-error";
                }
                div id="email-error" {}
            }

            div.form-group {
                label.form-label for="password" { "Password" }
                div.input-wrapper {
                    (svg::lock(20, 20))
                    input."form-input"#password type="password" name="password" placeholder="Create a strong password" autocomplete="new-password" required minlength="8";
                }
                p.form-hint { "Must be at least 8 characters" }
            }

            div.form-group {
                label.form-label for="confirm-password" { "Confirm Password" }
                div.input-wrapper {
                    (svg::lock(20, 20))
                    input."form-input" id="confirm-password" type="password" name="confirm_password" placeholder="Re-enter your password" autocomplete="new-password" required minlength="8";
                }
            }

            div.form-group {
                label.checkbox-label {
                    input.form-checkbox type="checkbox" name="terms" required;
                    span {
                        "I agree to the "
                        a.form-link href="/terms" target="_blank" { "Terms of Service" }
                        " and "
                        a.form-link href="/privacy" target="_blank" { "Privacy Policy" }
                    }
                }
            }

            button.button.button--primary.button--full type="submit" {
                span { "Create Account" }
                (svg::arrow_right(16, 16, 2))
            }
        }

        div.auth-footer {
            p.auth-footer__text {
                "Already have an account? "
                a.form-link href="/signin" { "Sign in" }
            }
        }
    }
}

pub(crate) fn verify_card() -> Markup {
    html! {
        div.auth-card__header {
            div.auth-logo {
                span.auth-logo__text { "AI" }
            }
            h1.auth-title { "Check Your Email" }
            p.auth-subtitle { "We've sent you a verification link" }
        }

        div.auth-form {
            div.verification-icon {
                (svg::envelope("", 64, 64))
            }

            div.verification-message {
                p.verification-message__main {
                    "We've sent a verification link to your email address. "
                    "Please check your inbox and click the link to verify your account."
                }

                div.verification-steps {
                    div.verification-step {
                        span.step-number { "1" }
                        span.step-text { "Open your email inbox" }
                    }
                    div.verification-step {
                        span.step-number { "2" }
                        span.step-text { "Find the verification email from RustGPT" }
                    }
                    div.verification-step {
                        span.step-number { "3" }
                        span.step-text { "Click the verification link" }
                    }
                }

                p.verification-message__note {
                    "Didn't receive the email? Check your spam folder."
                }
            }

            a.button.button--primary.button--full href=(crate::pages::signin::PATH) {
                span { "Back to Sign In" }
                (svg::arrow_right(16, 16, 2))
            }
        }

        div.auth-footer {
            p.auth-footer__text {
                "Need help? "
                a.form-link href=(format!("mailto:{TEAM_EMAIL}")) { "Contact support" }
            }
        }
    }
}

#[derive(Debug, Clone)]
enum VerificationStatus {
    Expired,
    Success,
    AlreadyVerified,
    Error,
}

fn verification_page(status: VerificationStatus) -> impl IntoResponse {
    use VerificationStatus::*;
    super::page(
        match status {
            Expired => "Link Expired",
            Success => "Verified",
            AlreadyVerified => "Already Verified",
            Error => "Unable to verify email",
        },
        html! {
            div.auth-container {
                div.auth-card {
                    div.auth-card__header {
                        div.auth-logo {
                            span.auth-logo__text { "AI" }
                        }
                        h1.auth-title { (match status {
                            Expired => "Verification Link Expired",
                            Success => "Email Verified!",
                            AlreadyVerified => "Email Already Verified",
                            Error => "Unable to verify email",
                        }) }
                        p.auth-subtitle { (match status {
                            Expired => "The verification link has expired",
                            Success => "Your account has been successfully verified",
                            AlreadyVerified => "Your email address has already been verified",
                            Error => "Unable to verify email. Please try signing up again.",
                        }) }
                    }

                    div.auth-form {
                        @match status {
                            Success => {
                                div.verification-icon.verification-icon--success {
                                    (svg::check_circle("", 64, 64))
                                }
                            }
                            Expired | Error => {
                                div.verification-icon.verification-icon--error {
                                    (svg::x_circle("", 64, 64))
                                }
                            }
                            AlreadyVerified => {
                                div.verification-icon.verification-icon--info {
                                    (svg::info_circle("", 64, 64))
                                }
                            }
                        }

                        div.verification-message {
                            @match status {
                                Success => {
                                    p.verification-message__main {
                                        "Great! Your email address has been verified. "
                                        "You're all set to start using " (crate::PROJECT_NAME) "."
                                    }
                                    p.verification-message__note {
                                        "You can now sign in with your credentials and start exploring."
                                    }
                                }
                                Expired => {
                                    p.verification-message__main {
                                        "This verification link has expired or is no longer valid. "
                                        "Please sign up again to receive a new verification link."
                                    }
                                    p.verification-message__note {
                                        (format!("Verification links expire after {} hours for security reasons.", VERIFICATION_LINK_EXPIRATION.num_hours()))
                                    }
                                }
                                AlreadyVerified => {
                                    p.verification-message__main {
                                        "Good news! Your email address has already been verified. "
                                        "You can sign in to your account right away."
                                    }
                                    p.verification-message__note {
                                        "No further action is needed on your part."
                                    }
                                }
                                Error => {
                                    p.verification-message__main {
                                        "Unable to verify email. Please try signing up again."
                                    }
                                    p.verification-message__note {
                                        "If the problem persists, please contact support."
                                    }
                                }
                            }
                        }

                        @match status {
                            Success | AlreadyVerified => {
                                a.button.button--primary.button--full href=(crate::pages::signin::PATH) {
                                    span { "Sign In to Your Account" }
                                    (svg::arrow_right(16, 16, 2))
                                }
                            }
                            Expired | Error => {
                                a.button.button--primary.button--full href=(PATH) {
                                    span { "Sign Up Again" }
                                    (svg::arrow_right(16, 16, 2))
                                }
                            }
                        }
                    }

                    div.auth-footer {
                        p.auth-footer__text {
                            "Need help? "
                            a.form-link href=(format!("mailto:{TEAM_EMAIL}")) { "Contact support" }
                        }
                    }
                }
            }
        },
    )
}

#[derive(Debug, Serialize, Deserialize)]
struct Link {
    user: User,
    created_at: DateTime<Utc>,
}

pub(crate) fn api() -> Router {
    Router::new()
        .route(
            "/signup/validate",
            get({
                #[derive(Debug, Deserialize)]
                struct ValidateEmailQuery {
                    email: EmailAddress,
                }
                async |Query(ValidateEmailQuery { email }): Query<ValidateEmailQuery>| {
                    trace!(%email, "email to check");
                    match email.validate() {
                        Ok(()) if mailer::is_sendable(&email).await => html! {},
                        Ok(()) => {
                            // check if email is already in use
                            match User::by_email(&email).await {
                                Ok(Some(_)) => html! {
                                    p.form-hint { "Email already in use" }
                                },
                                Ok(None) => html! {
                                    p.form-hint { "Email is available" }
                                },
                                Err(_) => html! {
                                    p.form-hint { "Failed to check email" }
                                },
                            }
                        }
                        Err(_validation_report) => html! {}, // user is still typing the email
                    }
                }
            }),
        )
        .route(
            "/signup",
            post({
                #[derive(Debug, Deserialize)]
                struct SignUpForm {
                    name: Name,
                    email: EmailAddress,
                    password: Password,
                    confirm_password: Password,
                }

                impl Validate for SignUpForm {
                    type Context = ();

                    fn validate_into(
                        &self,
                        ctx: &Self::Context,
                        mut parent: &mut dyn FnMut() -> garde::Path,
                        report: &mut garde::Report,
                    ) {
                        self.name
                            .validate_into(ctx, &mut nested_path!(parent, "name"), report);
                        self.email
                            .validate_into(ctx, &mut nested_path!(parent, "email"), report);

                        if self.password != self.confirm_password {
                            report.append(
                                nested_path!(parent, "confirm_password")(),
                                garde::Error::new("Passwords do not match"),
                            );
                        }

                        self.password.validate_into(
                            &crate::user::PasswordValidationContext {
                                email: self.email.clone(),
                                name: self.name.clone(),
                            },
                            &mut nested_path!(parent, "password"),
                            report,
                        );
                    }
                }

                async |ConnectInfo(socket_address): ConnectInfo<SocketAddr>,
                       Garde(Form(SignUpForm {
                           name,
                           email,
                           password,
                           confirm_password: _,
                       })): Garde<Form<SignUpForm>>| {
                    info!(%name, %email, %socket_address, "sign up requested");

                    // Check if email is already registered
                    if User::by_email(&email).await?.is_some() {
                        warn!(%email, "email already registered");
                        return AppResult::Ok(signup_card(true));
                    }

                    let user = User::new(name.clone(), email.clone(), password.clone()).await?;
                    let user_id = user.id;

                    mailer::send_email(
                        &email,
                        "Verify Your Email",
                        crate::auth::verification_email(
                            &name,
                            &format!(
                                "{}/api/signup/verify?token={}",
                                crate::PROJECT_URL,
                                GlassVault::new(Link {
                                    user,
                                    created_at: Utc::now(),
                                })?
                            ),
                        ),
                        lettre::message::header::ContentType::TEXT_HTML,
                    )
                    .await?;

                    info!(%user_id, %email, "sign up completed");
                    Ok(verify_card())
                }
            }),
        )
        .route(
            "/signup/verify",
            get({
                #[derive(Debug, Deserialize)]
                struct VerifyEmailQuery {
                    token: String,
                }
                async |Query(VerifyEmailQuery { token }): Query<VerifyEmailQuery>| {
                    let Link { user, created_at } = match GlassVault::<Link>::from_str(&token) {
                        Ok(container) => container.into_inner(),
                        Err(error) => {
                            error!(?error, "invalid email verification token");
                            return AppResult::Ok(verification_page(VerificationStatus::Expired));
                        }
                    };
                    let user_id = user.id;
                    let user_email = user.email.clone();

                    if Utc::now() - created_at > VERIFICATION_LINK_EXPIRATION {
                        warn!(%user_id, %user_email, "email verification link expired");
                        return AppResult::Ok(verification_page(VerificationStatus::Expired));
                    }
                    match user.save().await {
                        Ok(()) => Ok(verification_page(VerificationStatus::Success)),
                        Err(error) => {
                            use native_db::db_type::Error as NativeDbError;
                            if let Some(NativeDbError::DuplicateKey {
                                key_name: _key_name,
                            }) = error.downcast_ref::<NativeDbError>()
                            {
                                warn!(%user_id, %user_email, "user tried to verify again");
                                Ok(verification_page(VerificationStatus::AlreadyVerified))
                            } else {
                                error!(?error, %user_id, %user_email, "failed to save user");
                                Ok(verification_page(VerificationStatus::Error))
                            }
                        }
                    }
                }
            }),
        )
}
