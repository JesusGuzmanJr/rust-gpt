use {
    crate::http,
    axum::{
        Form, Router,
        extract::Query,
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
    },
    axum_extra::extract::CookieJar,
    chrono::{DateTime, Utc},
    chrono_tz::Tz,
    icu::locale::Locale,
    language_model::{
        message::UserMessage,
        models::{LANGUAGE_MODEL_0_INFO, LANGUAGE_MODEL_1_INFO, ModelInfo},
    },
    maud::{Markup, html},
    serde::Deserialize,
    strum::{Display, EnumIter, IntoEnumIterator},
    thousands::Separable,
};

pub(crate) async fn page(cookie_jar: CookieJar) -> impl IntoResponse {
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
                            h1.chat-header__title { "New Chat" }
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
                                div.message__wrapper {
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
                            }

                            // User message
                            (render_user_message(
                                &UserMessage::new("I need help with my project."),
                                Utc::now(),
                                &http::extract_locale(&cookie_jar),
                                &http::extract_timezone(&cookie_jar),
                            ))

                            // System message
                            div.message.message--system {
                                div.message__wrapper {
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
                    }

                    // Input area
                    div.chat-input {
                        div.chat-input__inner {
                            div.settings-popover {
                                button.chat-input__settings-btn id="settings-btn" {
                                    svg.icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                                        path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" {}
                                        circle cx="12" cy="12" r="3" {}
                                    }
                                }

                                div.popover-content id="settings-popover" {
                                    div.popover-inner {
                                        div.form-group {
                                            label.form-label { "Model" }
                                            select.form-select name="model" hx-get="/api/chat/models" hx-target=".model-details" {
                                                @for selection in ModelSelection::iter() {
                                                    option value=(selection) { (ModelInfo::from(selection).name) }
                                                }
                                            }
                                            div.model-details {
                                                (render_model_details(ModelSelection::default()))
                                            }
                                        }

                                        div.form-group {
                                            div.form-label-row {
                                                label.form-label { "Temperature" }
                                                span.form-value { "0.0" }
                                            }
                                            input.form-range type="range" min="-1" max="1" step="0.1" value="0" name="temperature";
                                            div.range-labels {
                                                span { "-1.0" }
                                                span { "0.0" }
                                                span { "1.0" }
                                            }
                                        }
                                    }
                                }
                            }

                            textarea.chat-input__textarea id="message-input" placeholder="Type your message..." rows="1" name="message" hx-post="/api/chat/send" hx-target=".chat-messages__inner" hx-swap="beforeend" hx-trigger="keydown[key=='Enter' && !shiftKey]" {}

                            button.button.button--primary.chat-input__send-btn id="send-btn" disabled hx-post="/api/chat/send" hx-target=".chat-messages__inner" hx-include="#message-input" hx-swap="beforeend"{
                                svg.icon width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" {
                                    path d="M5 12h14M12 5l7 7-7 7";
                                }
                            }
                        }
                    }
                }
            }
            script {(maud::PreEscaped(include_str!("chat.js")))
            }
        },
    )
}

pub(crate) fn api() -> Router {
    Router::new().nest(
        "/chat",
        Router::new()
            .route(
                "/models",
                get({
                    #[derive(Debug, Deserialize)]
                    struct ModelQuery {
                        // must match the "name" attribute
                        model: ModelSelection,
                    }

                    async |Query(ModelQuery { model }): Query<ModelQuery>| {
                        render_model_details(model).into_response()
                    }
                }),
            )
            .route(
                "/send",
                post({
                    #[derive(Debug, Deserialize)]
                    struct MessageQuery {
                        // must match the "name" attribute
                        message: String,
                    }

                    async |cookie_jar: CookieJar,
                           Form(MessageQuery { message }): Form<MessageQuery>| {
                        let _user_id = match http::extract_user_id(&cookie_jar) {
                            Some(user_id) => user_id,
                            None => {
                                return (StatusCode::BAD_REQUEST, "No user ID found")
                                    .into_response();
                            }
                        };
                        let user_message = UserMessage::new(message);
                        let _thread_id = match http::extract_thread_id(&cookie_jar) {
                            Some(thread_id) => thread_id,
                            None => {
                                return (StatusCode::BAD_REQUEST, "No thread ID found")
                                    .into_response();
                            }
                        };

                        render_user_message(
                            &user_message,
                            Utc::now(),
                            &http::extract_locale(&cookie_jar),
                            &http::extract_timezone(&cookie_jar),
                        )
                        .into_response()
                    }
                }),
            ),
    )
}

#[derive(Debug, Display, Default, EnumIter, Deserialize)]
enum ModelSelection {
    #[default]
    Model0,
    Model1,
}

impl From<ModelSelection> for ModelInfo {
    fn from(model_option: ModelSelection) -> Self {
        match model_option {
            ModelSelection::Model0 => LANGUAGE_MODEL_0_INFO,
            ModelSelection::Model1 => LANGUAGE_MODEL_1_INFO,
        }
    }
}

fn render_model_details(selection: ModelSelection) -> Markup {
    let model_info: ModelInfo = selection.into();
    html! {
        div.model-detail { (format!("Corpus Size: {}", model_info.corpus_size.display().iec())) }
        div.model-detail { (format!("Vocabulary Size: {}", model_info.vocabulary_size.separate_with_commas())) }
    }
}

fn render_user_message(
    user_message: &UserMessage,
    datetime: DateTime<Utc>,
    locale: &Locale,
    timezone: &Tz,
) -> Markup {
    // Note the user message needs to be escaped; htmx escapes by default.
    html! {
        div.message.message--user {
            div.message__wrapper {
                div.message__bubble.message__bubble--user {
                    p { (user_message) }
                }
                div.message__meta.message__meta--user {
                    span.message__time { (crate::datetime::today_implied_human_datetime(&datetime, &locale, &timezone)) }
                    button.message__edit-btn {
                        svg.icon.icon--sm width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
                            path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z";
                            path d="m15 5 4 4";
                        }
                    }
                }
            }
        }
    }
}
