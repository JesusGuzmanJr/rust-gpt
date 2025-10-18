use {axum::response::IntoResponse, maud::html};

pub(crate) async fn chat() -> impl IntoResponse {
    super::page(
        "Chat",
        html! {
            div.chat-container {
                // Sidebar
                aside.chat-sidebar {
                    div.chat-sidebar__header {
                        button.button.button--primary.chat-sidebar__new-btn {
                            svg.icon width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                path d="M12 5v14M5 12h14";
                            }
                            span { "New Chat" }
                        }
                    }

                    div.chat-sidebar__list {
                        div.chat-item.chat-item--active {
                            div.chat-item__content {
                                div.chat-item__header {
                                    svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                        path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z";
                                    }
                                    span.chat-item__title { "Project Help" }
                                    span.chat-item__time { "1h ago" }
                                }
                                p.chat-item__preview { "I need help with my project." }
                            }
                        }

                        div.chat-item {
                            div.chat-item__content {
                                div.chat-item__header {
                                    svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                        path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z";
                                    }
                                    span.chat-item__title { "Code Review" }
                                    span.chat-item__time { "2h ago" }
                                }
                                p.chat-item__preview { "Can you review this code?" }
                            }
                        }

                        div.chat-item {
                            div.chat-item__content {
                                div.chat-item__header {
                                    svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                        path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z";
                                    }
                                    span.chat-item__title { "Design Feedback" }
                                    span.chat-item__time { "1d ago" }
                                }
                                p.chat-item__preview { "What do you think about this design?" }
                            }
                        }
                    }

                    div.chat-sidebar__footer {
                        a.chat-sidebar__link href="/about" {
                            svg.icon.icon--xs width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                circle cx="12" cy="12" r="10";
                                path d="M12 16v-4M12 8h.01";
                            }
                            span { "About" }
                        }
                    }
                }

                // Main content
                div.chat-main {
                    // Header
                    header.chat-header {
                        div.chat-header__left {
                            div.chat-header__logo {
                                span { "AI" }
                            }
                            h1.chat-header__title { "AI Chat" }
                        }

                        div.chat-header__right {
                            button.chat-header__user-btn {
                                span { "user@example.com" }
                            }
                        }
                    }

                    // Messages area
                    main.chat-messages {
                        div.chat-messages__inner {
                            // System message
                            div.message.message--system {
                                div.message__bubble.message__bubble--system {
                                    p { "Hello! How can I assist you today?" }
                                }
                                div.message__meta {
                                    span.message__time { "2:30 PM" }
                                    button.message__feedback-btn {
                                        svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                            path d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3";
                                        }
                                    }
                                    button.message__feedback-btn {
                                        svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                            path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7-13h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17";
                                        }
                                    }
                                }
                            }

                            // User message
                            div.message.message--user {
                                div.message__bubble.message__bubble--user {
                                    p { "I need help with my project." }
                                }
                                div.message__meta.message__meta--user {
                                    span.message__time { "2:31 PM" }
                                    button.message__edit-btn {
                                        svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                            path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7";
                                            path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z";
                                        }
                                    }
                                }
                            }

                            // System message
                            div.message.message--system {
                                div.message__bubble.message__bubble--system {
                                    p { "I'd be happy to help! Could you tell me more about your project and what specific assistance you need?" }
                                }
                                div.message__meta {
                                    span.message__time { "2:31 PM" }
                                    button.message__feedback-btn {
                                        svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                            path d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3";
                                        }
                                    }
                                    button.message__feedback-btn {
                                        svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                            path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7-13h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17";
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Input area
                    div.chat-input {
                        div.chat-input__inner {
                            button.chat-input__settings-btn {
                                svg.icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                    path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" {}
                                    circle cx="12" cy="12" r="3" {}
                                }
                            }

                            textarea.chat-input__textarea placeholder="Type your message..." rows="1" {}

                            button.button.button--primary.chat-input__send-btn href="/#foo" {
                                svg.icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" {
                                    path d="M5 12h14M12 5l7 7-7 7";
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}
