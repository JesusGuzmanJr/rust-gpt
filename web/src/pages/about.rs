use {axum::response::IntoResponse, maud::html};

pub(crate) const PATH: &str = "/about";

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "About",
        html! {
            div.legal-container {
                div.legal-card {
                    div.legal-header {
                        h1.legal-title { "About This Project" }
                    }

                    div.legal-content {
                        section.legal-section {
                            h2 { "Purpose" }
                            p {
                                "This project's mission is to push boundaries: How much can a developer "
                                "design, train, and operate a chat-based GPT model entirely from scratch? "
                                "The focus is on exploring the limits of what can be accomplished, "
                                "using Rust as the core tech. "
                                "(With a sprinkle of CUDA 😃)"
                            }
                        }

                        section.legal-section {
                            h2 { "Technology Stack" }
                            p {
                                "This application is built with Rust because its type system allows me to model "
                                "problems that would require too much boilerplate, comments, or linter checks "
                                " in other mainstream languages like Python or TypeScript."
                            }
                        }

                        section.legal-section {
                            h2 { "Experimental Nature" }
                            p {
                                "As an experimental project, features are all alpha, may change "
                                "without notice, or occasionally not work at all during planned "
                                "and unplanned downtime. "
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
