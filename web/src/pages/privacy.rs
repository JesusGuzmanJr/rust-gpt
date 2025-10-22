use {axum::response::IntoResponse, maud::html};

pub(crate) async fn page() -> impl IntoResponse {
    super::page(
        "Privacy Policy",
        html! {
            div.legal-container {
                div.legal-card {
                    div.legal-header {
                        h1.legal-title { "Privacy Policy" }
                        p.legal-date { "Last Updated: October 22, 2025" }
                    }

                    div.legal-content {
                        section.legal-section {
                            h2 { "1. Introduction" }
                            p {
                                "This Privacy Policy explains how we collect, use, and protect your personal information "
                                "when you use our experimental AI chat service. We are committed to being transparent about "
                                "our data practices."
                            }
                        }

                        section.legal-section {
                            h2 { "2. Information We Collect" }
                            h3.legal-subsection { "2.1 Account Information" }
                            p { "When you create an account, we collect:" }
                            ul.legal-list {
                                li { "Email address" }
                                li { "Password (stored securely using industry-standard hashing)" }
                                li { "Account creation date and time" }
                            }

                            h3.legal-subsection { "2.2 Usage Data" }
                            p { "During your use of the service, we collect:" }
                            ul.legal-list {
                                li { "Chat messages and conversations" }
                                li { "Timestamps of interactions" }
                                li { "Session information" }
                                li { "Technical data such as IP address and browser information" }
                            }
                        }

                        section.legal-section {
                            h2 { "3. How We Use Your Information" }
                            p { "We use the collected information for the following purposes:" }
                            ul.legal-list {
                                li { "Providing and maintaining the service" }
                                li { "Analytical and educational research" }
                                li { "Product development and improvement" }
                                li { "Understanding usage patterns and user behavior" }
                                li { "Troubleshooting technical issues" }
                            }
                        }

                        section.legal-section {
                            h2 { "4. Data Storage and Location" }
                            p {
                                "All data is stored on servers located in Seattle, Washington. We implement appropriate "
                                "technical and organizational measures to protect your data against unauthorized access, "
                                "alteration, disclosure, or destruction."
                            }
                        }

                        section.legal-section {
                            h2 { "5. Developer Access to Data" }
                            p.legal-important {
                                strong { "Important: " }
                                "Your email address and all content you provide will remain visible to the developers "
                                "for product research purposes. This is an experimental service, and developer access "
                                "to user data is necessary for research, debugging, and improvement of the platform."
                            }
                        }

                        section.legal-section {
                            h2 { "6. Third-Party Services" }
                            p {
                                "We do NOT use any third-party trackers, analytics services, or advertising networks. "
                                "Your data is not shared with or sold to any third parties for marketing or advertising purposes."
                            }
                        }

                        section.legal-section {
                            h2 { "7. Data Retention" }
                            p {
                                "We retain your personal information and usage data for as long as your account is active "
                                "or as needed for our analytical and educational purposes. You may request deletion of "
                                "your account and associated data at any time."
                            }
                        }

                        section.legal-section {
                            h2 { "8. Your Rights" }
                            p { "You have the right to:" }
                            ul.legal-list {
                                li { "Access your personal information" }
                                li { "Request correction of inaccurate data" }
                                li { "Request deletion of your account and data" }
                                li { "Export your conversation data" }
                                li { "Withdraw consent for data processing (by deleting your account)" }
                            }
                        }

                        section.legal-section {
                            h2 { "9. Security Measures" }
                            p {
                                "We implement security measures including encrypted passwords, secure HTTPS connections, "
                                "and access controls. However, no method of transmission over the internet or electronic "
                                "storage is 100% secure, and we cannot guarantee absolute security."
                            }
                        }

                        section.legal-section {
                            h2 { "10. Children's Privacy" }
                            p {
                                "This service is not intended for users under the age of 13. We do not knowingly collect "
                                "personal information from children under 13. If you become aware that a child has provided "
                                "us with personal information, please contact us."
                            }
                        }

                        section.legal-section {
                            h2 { "11. Changes to This Privacy Policy" }
                            p {
                                "We may update our Privacy Policy from time to time. We will notify you of any changes "
                                "by posting the new Privacy Policy on this page and updating the \"Last Updated\" date."
                            }
                        }

                        section.legal-section {
                            h2 { "12. Experimental Nature" }
                            p {
                                "Please be aware that this is an experimental service. We recommend not storing sensitive, "
                                "confidential, or personally identifiable information in your conversations. Use this "
                                "service at your own risk."
                            }
                        }

                        section.legal-section {
                            h2 { "13. Contact Us" }
                            p {
                                "If you have any questions about this Privacy Policy or wish to exercise your rights "
                                "regarding your personal data, please contact the development team through the appropriate "
                                "channels provided in the service."
                            }
                        }
                    }

                    div.legal-footer {
                        p {
                            a.legal-link href="/terms" { "Terms and Conditions" }
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
