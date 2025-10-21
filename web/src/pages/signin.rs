use {
    axum::{
        Form, Router,
        http::StatusCode,
        response::{IntoResponse, Redirect},
        routing::post,
    },
    axum_extra::extract::CookieJar,
    maud::html,
    serde::Deserialize,
};

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

                    form.auth-form method="post" action="/api/auth/signin" {
                        div.form-group {
                            label.form-label for="email" { "Email" }
                            div.input-wrapper {
                                svg.input-icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
                                    rect x="3" y="5" width="18" height="14" rx="2" {}
                                    path d="m3 7 9 6 9-6" {}
                                }
                                input."form-input"#email type="email" name="email" placeholder="you@example.com" required autofocus;
                            }
                        }

                        div.form-group {
                            label.form-label for="password" { "Password" }
                            div.input-wrapper {
                                svg.input-icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
                                    rect x="5" y="11" width="14" height="10" rx="2" ry="2" {}
                                    path d="M7 11V7a5 5 0 0 1 10 0v4" {}
                                }
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
                            svg.icon width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                path d="M5 12h14M12 5l7 7-7 7";
                            }
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
            #[derive(Debug, Deserialize)]
            struct SignInForm {
                email: String,
                password: String,
                remember: Option<String>,
            }

            async |_cookie_jar: CookieJar,
                   Form(SignInForm {
                       email,
                       password,
                       remember: _remember,
                   }): Form<SignInForm>| {
                // TODO: Implement actual authentication logic
                // For now, this is a placeholder

                // Validate credentials (placeholder)
                if email.is_empty() || password.is_empty() {
                    return (StatusCode::BAD_REQUEST, "Invalid credentials").into_response();
                }

                // Set user session cookies
                // let cookie_jar = http::set_user_id(cookie_jar, user_id);
                // let cookie_jar = http::set_thread_id(cookie_jar, thread_id);

                Redirect::to("/chat").into_response()
            }
        }),
    )
}
