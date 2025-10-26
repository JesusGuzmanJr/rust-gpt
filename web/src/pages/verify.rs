use {crate::svg, axum::response::IntoResponse, maud::html};

pub(crate) const PATH: &str = "/verify";

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "Verify Email",
        html! {
            div.auth-container {
                div.auth-card {
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
                                "Didn't receive the email? Check your spam folder or "
                                a.form-link href="/resend-verification" { "resend the verification email" }
                                "."
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
                            a.form-link href="/support" { "Contact support" }
                        }
                    }
                }
            }
        },
    )
}
