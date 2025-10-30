use {crate::TEAM_EMAIL, axum::response::IntoResponse, maud::html};

pub(crate) const PATH: &str = "/terms";

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "Terms and Conditions",
        html! {
            div.legal-container {
                div.legal-card {
                    div.legal-header {
                        h1.legal-title { "Terms and Conditions" }
                        p.legal-date { "Last Updated: October 22, 2025" }
                    }

                    div.legal-content {
                        section.legal-section {
                            h2 { "1. Acceptance of Terms" }
                            p {
                                "By accessing and using this service, you accept and agree to be bound by the terms "
                                "and provision of this agreement. This is an experimental platform developed for "
                                "analytical and educational purposes."
                            }
                        }

                        section.legal-section {
                            h2 { "2. Experimental Nature" }
                            p {
                                "This service is provided on an experimental basis. We do not guarantee the availability, "
                                "accuracy, or reliability of the service. Features may change without notice, and the "
                                "service may be discontinued at any time."
                            }
                        }

                        section.legal-section {
                            h2 { "3. Data Collection and Storage" }
                            p {
                                "All information collected through this service is stored in Seattle, Washington. "
                                "The data is used exclusively for analytical and educational purposes. We do not use "
                                "any third-party trackers or analytics services."
                            }
                            p {
                                "Your email address and any content you provide will be visible to the developers "
                                "for product research purposes. By using this service, you consent to this data access."
                            }
                        }

                        section.legal-section {
                            h2 { "4. User Responsibilities" }
                            p { "As a user of this service, you agree to:" }
                            ul.legal-list {
                                li { "Provide accurate information during registration" }
                                li { "Maintain the security of your account credentials" }
                                li { "Use the service for lawful purposes only" }
                                li { "Not attempt to circumvent security measures or access unauthorized areas" }
                                li { "Not use the service to store sensitive or confidential information" }
                            }
                        }

                        section.legal-section {
                            h2 { "5. Intellectual Property" }
                            p {
                                "The service and its original content, features, and functionality are owned by the "
                                "developers and are protected by international copyright, trademark, patent, trade secret, "
                                "and other intellectual property laws."
                            }
                        }

                        section.legal-section {
                            h2 { "6. Disclaimer of Warranties" }
                            p {
                                "This service is provided \"AS IS\" and \"AS AVAILABLE\" without any warranties of any kind, "
                                "either express or implied. We do not warrant that the service will be uninterrupted, "
                                "secure, or error-free."
                            }
                        }

                        section.legal-section {
                            h2 { "7. Limitation of Liability" }
                            p {
                                "In no event shall the developers be liable for any indirect, incidental, special, "
                                "consequential, or punitive damages resulting from your use or inability to use the service."
                            }
                        }

                        section.legal-section {
                            h2 { "8. Account Termination" }
                            p {
                                "We reserve the right to terminate or suspend access to the service immediately, "
                                "without prior notice or liability, for any reason whatsoever, including without "
                                "limitation if you breach the Terms."
                            }
                        }

                        section.legal-section {
                            h2 { "9. Changes to Terms" }
                            p {
                                "We reserve the right to modify or replace these Terms at any time. It is your "
                                "responsibility to check these Terms periodically for changes. Your continued use "
                                "of the service following the posting of any changes constitutes acceptance of those changes."
                            }
                        }

                        section.legal-section {
                            h2 { "10. Contact Information" }
                            p {
                                "If you have any questions about these Terms, please email me at:"
                                a.legal-link href=(format!("mailto:{TEAM_EMAIL}")) { (TEAM_EMAIL) }
                            }
                        }
                    }

                    div.legal-footer {
                        p {
                            a.legal-link href="/privacy" { "Privacy Policy" }
                            " | "
                            a.legal-link href="/signup" { "Back to Sign Up" }
                            " | "
                            a.legal-link href="/signin" { "Sign In" }
                        }
                    }
                }
            }
        },
    )
}
