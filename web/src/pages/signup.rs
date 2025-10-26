use {
    crate::{
        error::AppResult,
        hash::Container,
        mailer, svg,
        user::{EmailAddress, Name, Password, User},
    },
    axum::{
        Form, Router,
        extract::Query,
        response::{IntoResponse, Redirect},
        routing::{get, post},
    },
    axum_extra::extract::CookieJar,
    axum_valid::Garde,
    garde::{Validate, util::nested_path},
    maud::html,
    serde::Deserialize,
    tracing::*,
};

pub(crate) const PATH: &str = "/signup";

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "Sign Up",
        html! {
            div.auth-container {
                div.auth-card {
                    div.auth-card__header {
                        div.auth-logo {
                            span.auth-logo__text { "AI" }
                        }
                        h1.auth-title { "Create Account" }
                        p.auth-subtitle { "Sign up to get started with " (crate::PROJECT_NAME) }
                    }

                    form.auth-form hx-post="/api/signup" {
                        div.form-group {
                            label.form-label for="name" { "Full Name" }
                            div.input-wrapper {
                                (svg::user(20, 20))
                                input."form-input"#name type="text" name="name" placeholder="Jane Doe" required autofocus;
                            }
                        }

                        div.form-group {
                            label.form-label for="email" { "Email" }
                            div.input-wrapper {
                                (svg::mail(20, 20))
                                input."form-input"#email
                                    type="email"
                                    name="email"
                                    placeholder="you@example.com"
                                    required
                                    hx-get="/api/signup/validate-email"
                                    hx-trigger="keyup changed delay:500ms"
                                    hx-target="#email-error";
                            }
                            div id="email-error" {}
                        }

                        div.form-group {
                            label.form-label for="password" { "Password" }
                            div.input-wrapper {
                                (svg::lock(20, 20))
                                input."form-input"#password type="password" name="password" placeholder="Create a strong password" required minlength="8";
                            }
                            p.form-hint { "Must be at least 8 characters" }
                        }

                        div.form-group {
                            label.form-label for="confirm-password" { "Confirm Password" }
                            div.input-wrapper {
                                (svg::lock(20, 20))
                                input."form-input" id="confirm-password" type="password" name="confirm_password" placeholder="Re-enter your password" required minlength="8";
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
        },
    )
}

pub(crate) fn api() -> Router {
    Router::new()
        .route(
            "/signup/validate-email",
            get({
                #[derive(Debug, Deserialize)]
                struct ValidateEmailQuery {
                    email: EmailAddress,
                }
                async |Query(ValidateEmailQuery { email }): Query<ValidateEmailQuery>| {
                    trace!(%email, "email to check");
                    match email.validate() {
                        Ok(()) if mailer::is_sendable(&email).await => html! {},
                        Err(_) => html! {}, // user is still typing the email
                        _ => {
                            html! {
                                p.form-hint { "Please enter a valid email address" }
                            }
                        }
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

                async |_cookie_jar: CookieJar,
                       Garde(Form(SignUpForm {
                           name,
                           email,
                           password,
                           confirm_password: _,
                       })): Garde<Form<SignUpForm>>| {
                    info!(%name, %email, "sign up requested");

                    let user = User::new(name, email, password)?;
                    let _user_id = user.id;
                    let _email = user.email.clone();

                    let _verify_link = format!(
                        "{}/api/signup/verify?token={}",
                        crate::PROJECT_URL,
                        Container::new(user)?
                    );

                    // TODO: Send verification email with _verify_link
                    // TODO: Check if email is already in use

                    // Redirect to verify page
                    info!(%_user_id, %_email, "sign up completed");

                    AppResult::Ok(Redirect::to(crate::pages::verify::PATH).into_response())
                }
            }),
        )
}
