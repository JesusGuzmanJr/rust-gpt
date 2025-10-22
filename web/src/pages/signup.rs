use {
    crate::user::{EmailAddress, Name, Password, User},
    axum::{
        Form, Router,
        http::StatusCode,
        response::{IntoResponse, Redirect},
        routing::post,
    },
    axum_extra::extract::CookieJar,
    axum_valid::Garde,
    garde::{Validate, util::nested_path},
    maud::html,
    serde::Deserialize,
    tracing::debug,
};

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
                        p.auth-subtitle { "Sign up to get started with RustGPT" }
                    }

                    form.auth-form method="post" action="/api/signup" {
                        div.form-group {
                            label.form-label for="name" { "Full Name" }
                            div.input-wrapper {
                                svg.input-icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
                                    path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" {}
                                    circle cx="12" cy="7" r="4" {}
                                }
                                input."form-input"#name type="text" name="name" placeholder="John Doe" required autofocus;
                            }
                        }

                        div.form-group {
                            label.form-label for="email" { "Email" }
                            div.input-wrapper {
                                svg.input-icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
                                    rect x="3" y="5" width="18" height="14" rx="2" {}
                                    path d="m3 7 9 6 9-6" {}
                                }
                                input."form-input"#email type="email" name="email" placeholder="you@example.com" required;
                            }
                        }

                        div.form-group {
                            label.form-label for="password" { "Password" }
                            div.input-wrapper {
                                svg.input-icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
                                    rect x="5" y="11" width="14" height="10" rx="2" ry="2" {}
                                    path d="M7 11V7a5 5 0 0 1 10 0v4" {}
                                }
                                input."form-input"#password type="password" name="password" placeholder="Create a strong password" required minlength="8";
                            }
                            p.form-hint { "Must be at least 8 characters" }
                        }

                        div.form-group {
                            label.form-label for="confirm-password" { "Confirm Password" }
                            div.input-wrapper {
                                svg.input-icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
                                    rect x="5" y="11" width="14" height="10" rx="2" ry="2" {}
                                    path d="M7 11V7a5 5 0 0 1 10 0v4" {}
                                }
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
                            svg.icon width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                path d="M5 12h14M12 5l7 7-7 7";
                            }
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
    Router::new().route(
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
                       confirm_password,
                   })): Garde<Form<SignUpForm>>| {
                debug!(%name, %email, "sign up");

                // create user with async fn; error should be enum to handle unique constraint
                // violations
                Redirect::to("/chat").into_response()
            }
        }),
    )
}
