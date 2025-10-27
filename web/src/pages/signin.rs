use {
    crate::{
        auth,
        error::AppResult,
        svg,
        user::{EmailAddress, Password, User},
    },
    axum::{
        Form, Router,
        response::{IntoResponse, Redirect},
        routing::post,
    },
    axum_extra::extract::CookieJar,
    axum_valid::Garde,
    garde::Validate,
    maud::{Markup, html},
    serde::Deserialize,
    tracing::*,
};

pub(crate) const PATH: &str = "/signin";

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "Sign In",
        html! {
            div.auth-container {
                div.auth-card {
                    (signin_card(false))
                }
            }
        },
    )
}

fn signin_card(show_error: bool) -> Markup {
    html! {
        div.auth-card__header {
            div.auth-logo {
                span.auth-logo__text { "AI" }
            }
            h1.auth-title { "Welcome Back" }
            p.auth-subtitle { "Sign in to continue to your account" }
        }

        form.auth-form hx-post="/api/signin" hx-target=".auth-card" {
            @if show_error {
                div.auth-error {
                    (svg::x_circle("auth-error__icon", 20, 20))
                    span.auth-error__text { "Invalid email or password" }
                }
            }

            div.form-group {
                label.form-label for="email" { "Email" }
                div.input-wrapper {
                    (svg::envelope("input-icon", 20, 20))
                    input."form-input"#email type="email" name="email" placeholder="you@example.com" required autofocus;
                }
            }

            div.form-group {
                label.form-label for="password" { "Password" }
                div.input-wrapper {
                    (svg::lock(20, 20))
                    input."form-input"#password type="password" name="password" placeholder="Enter your password" required;
                }
            }

            div.form-options {
                label.checkbox-label {
                    input.form-checkbox type="checkbox" name="remember";
                    span { "Remember me" }
                }
                a.form-link href="/forgot-password" { "Forgot password?" }
            }

            button.button.button--primary.button--full type="submit" {
                span { "Sign In" }
                (svg::arrow_right(16, 16, 2))
            }
        }

        div.auth-footer {
            p.auth-footer__text {
                "Don't have an account? "
                a.form-link href="/signup" { "Sign up" }
            }
        }
    }
}

pub(crate) fn api() -> Router {
    Router::new().route(
        "/signin",
        post({
            #[derive(Debug, Deserialize, Validate)]
            struct SignInForm {
                #[garde(dive)]
                email: EmailAddress,
                #[garde(skip)]
                password: Password,
                #[garde(skip)]
                remember: Option<String>,
            }

            async |cookie_jar: CookieJar,
                   Garde(Form(SignInForm {
                       email,
                       password,
                       remember: _remember,
                   })): Garde<Form<SignInForm>>| {
                info!(%email, "sign in requested");

                let user = match User::by_email(&email).await? {
                    Some(user) => user,
                    None => {
                        warn!(%email, "user not found");
                        return AppResult::Ok(signin_card(true).into_response());
                    }
                };

                let user_id = user.id;

                // consumes user because needs to move to CPU-bound thread pool
                if !user.verify_password(password).await? {
                    warn!(%user_id, "invalid password");
                    return AppResult::Ok(signin_card(true).into_response());
                }

                let cookie_jar = auth::create_auth_cookie(cookie_jar, user_id)?;

                info!(%user_id, "sign in successful");
                AppResult::Ok((cookie_jar, Redirect::to(crate::pages::chat::PATH)).into_response())
            }
        }),
    )
}
