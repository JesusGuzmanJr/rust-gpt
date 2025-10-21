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
        "Sign Up",
        html! {
            div.auth-container {
                div.auth-card {
                    div.auth-card__header {
                        div.auth-logo {
                            span.auth-logo__text { "AI" }
                        }
                        h1.auth-title { "Create Account" }
                        p.auth-subtitle { "Sign up to get started with AI Chat" }
                    }

                    form.auth-form method="post" action="/api/auth/signup" {
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
                name: String,
                email: String,
                password: String,
                confirm_password: String,
                terms: Option<String>,
            }

            async |_cookie_jar: CookieJar,
                   Form(SignUpForm {
                       name,
                       email,
                       password,
                       confirm_password,
                       terms,
                   }): Form<SignUpForm>| {
                // TODO: Implement actual registration logic
                // For now, this is a placeholder

                // Validate inputs
                if name.is_empty() || email.is_empty() || password.is_empty() {
                    return (StatusCode::BAD_REQUEST, "All fields are required").into_response();
                }

                if password != confirm_password {
                    return (StatusCode::BAD_REQUEST, "Passwords do not match").into_response();
                }

                if password.len() < 8 {
                    return (
                        StatusCode::BAD_REQUEST,
                        "Password must be at least 8 characters",
                    )
                        .into_response();
                }

                if terms.is_none() {
                    return (
                        StatusCode::BAD_REQUEST,
                        "You must agree to the terms of service",
                    )
                        .into_response();
                }

                // Create user account (placeholder)
                // let user_id = create_user(&name, &email, &password)?;

                // Set user session cookies
                // let cookie_jar = http::set_user_id(cookie_jar, user_id);
                // let cookie_jar = http::set_thread_id(cookie_jar, thread_id);

                Redirect::to("/chat").into_response()
            }
        }),
    )
}
