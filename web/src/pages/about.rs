use {crate::PROJECT_NAME, axum::response::IntoResponse, maud::html};

pub(crate) const PATH: &str = "/about";

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "About",
        html! {
            div.legal-container {
                div.legal-card {
                    div.legal-header {
                        h1.legal-title { (PROJECT_NAME) }
                    }

                    div.legal-subtitle {
                        p {
                            "Created by Jesus Guzman, Jr."
                        }
                        p {
                            a.legal-link href="https://github.com/JesusGuzmanJr" { "GitHub" }
                            " | "
                            a.legal-link href="https://www.linkedin.com/in/jesusguzmanjr/" { "LinkedIn" }
                        }
                    }

                    div.legal-content {
                        section.legal-section {
                            h2 { "Why make this?" }
                            p {
                                "This project's mission is to push personal boundaries: How much can I "
                                "design, train, and operate a chat-capable GPT model from scratch? "
                                "Just how rudimentary would it even be? "
                                "The focus is on exploring the limits of what can be accomplished, "
                                "using Rust as the core tech. "
                                "(With a sprinkle of CUDA 😃)"
                            }
                        }

                        section.legal-section {
                            h2 { "Why Rust?" }
                            p {
                                "This application is built with Rust because its type system allows me to model "
                                "problems that would require too much boilerplate, comments, or linter checks "
                                " in other mainstream languages like Python or TypeScript."
                            }
                        }

                        section.legal-section {
                            h2 { "Beware what?" }
                            p {
                                "As a pet project, features are all alpha, "
                                span.strikethrough { "may" }
                                " "
                                i { "will" }
                                " change without notice, or occasionally not work at all. "
                                "Oh, and there are abuse controls in place too."
                            }
                        }
                    }

                    div.legal-footer {
                        p {
                            a.legal-link href="/" { "Home" }
                            " | "
                            a.legal-link href="/privacy" { "Privacy Policy" }
                            " | "
                            a.legal-link href="/terms" { "Terms" }
                        }
                    }
                }
            }
        },
    )
}
