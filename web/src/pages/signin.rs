use {
    crate::{
        svg,
        user::{EmailAddress, Password},
    },
    axum::{
        Form, Router,
        http::StatusCode,
        response::{IntoResponse, Redirect},
        routing::post,
    },
    axum_extra::extract::CookieJar,
    axum_valid::Garde,
    garde::Validate,
    maud::html,
    serde::Deserialize,
};

pub(crate) const PATH: &str = "/signin";

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "Sign In",
        html! {
            div.auth-container {
                div.auth-card {
                    div.auth-card__header {
                        div.auth-logo {
                            span.auth-logo__text { "AI" }
                        }
                        h1.auth-title { "Welcome Back" }
                        p.auth-subtitle { "Sign in to continue to your account" }
                    }

                    form.auth-form method="post" action="/api/signin" {
                        div.form-group {
                            label.form-label for="email" { "Email" }
                            div.input-wrapper {
                                (svg::mail(20, 20))
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
        },
    )
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

            async |_cookie_jar: CookieJar,
                   Garde(Form(SignInForm {
                       email,
                       password,
                       remember,
                   })): Garde<Form<SignInForm>>| {
                let remember = remember.is_some();
                dbg!(&email, &password, &remember);
                // TODO: Implement actual authentication logic
                // For now, this is a placeholder

                // Validate credentials (placeholder)

                // Set user session cookies
                // let cookie_jar = http::set_user_id(cookie_jar, user_id);
                // let cookie_jar = http::set_thread_id(cookie_jar, thread_id);

                Redirect::to("/chat").into_response()
            }
        }),
    )
}
