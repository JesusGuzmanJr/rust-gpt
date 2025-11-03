use {
    crate::{
        auth::require_auth_user,
        error::{AppResult, ResponseResult},
        http::extract,
        internationalization::Internationalization,
        message::{Feedback, Message, Payload, UserMessageContent},
        svg,
        thread::{Thread, ThreadId, ThreadTitle},
        user::UserId,
    },
    axum::{
        Form, Router,
        extract::{FromRequestParts, Query},
        http::{StatusCode, request::Parts},
        response::{IntoResponse, Redirect},
        routing::{get, post},
    },
    axum_extra::extract::CookieJar,
    axum_valid::Garde,
    garde::Validate,
    language_model::models::{LANGUAGE_MODEL_0_INFO, LANGUAGE_MODEL_1_INFO, ModelInfo},
    maud::{Markup, html},
    serde::Deserialize,
    std::{cmp::Reverse, convert::Infallible},
    strum::{Display, EnumIter, IntoEnumIterator},
    thousands::Separable,
    tracing::*,
};

pub(crate) const PATH: &str = "/chat";

pub(crate) async fn page(
    internationalization: Internationalization,
    cookie_jar: CookieJar,
) -> ResponseResult {
    tracing::info!("chat page requested");
    let user = require_auth_user(&cookie_jar).await?;

    let threads = {
        let mut threads = Thread::get_all(user.id).await?;
        threads.sort_unstable_by_key(|t| Reverse(t.created_at));
        threads
    };

    let messages = {
        if let Some(thread) = threads.first() {
            let mut messages = Message::get_all_messages(thread.id).await?;
            messages.sort_unstable_by_key(|m| Reverse(m.created_at));
            messages
        } else {
            Vec::with_capacity(0)
        }
    };

    let current_thread_title = threads
        .first()
        .map(|t| t.thread_title.clone())
        .unwrap_or_else(ThreadTitle::new_chat_title);

    Ok(super::page(
        "Chat",
        html! {
            div.chat-container {
                // Sidebar
                aside.chat-sidebar {
                    div.chat-sidebar__header {
                        button.button.button--primary.chat-sidebar__new-btn {
                            (svg::plus(16, 16))
                            span { "New Chat" }
                        }
                    }

                    div.chat-sidebar__list {
                        div.chat-item.chat-item--active {
                            div.chat-item__content {
                                div.chat-item__header {
                                    (svg::chat_bubble(16, 16))
                                    span.chat-item__title { "Project Help" }
                                    span.chat-item__time { "1h ago" }
                                }
                                p.chat-item__preview { "I need help with my project." }
                            }
                        }

                        div.chat-item {
                            div.chat-item__content {
                                div.chat-item__header {
                                    (svg::chat_bubble(16, 16))
                                    span.chat-item__title { "Code Review" }
                                    span.chat-item__time { "2h ago" }
                                }
                                p.chat-item__preview { "Can you review this code?" }
                            }
                        }

                        div.chat-item {
                            div.chat-item__content {
                                div.chat-item__header {
                                    (svg::chat_bubble(16, 16))
                                    span.chat-item__title { "Design Feedback" }
                                    span.chat-item__time { "1d ago" }
                                }
                                p.chat-item__preview { "What do you think about this design?" }
                            }
                        }
                    }

                    div.chat-sidebar__footer {
                        a.chat-sidebar__link href="/about" {
                            (svg::info_circle("icon icon--xs", 12, 12))
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
                            // Title display mode
                            h1.chat-header__title id="chat-title-display" { (current_thread_title) }

                            // Title edit mode (initially hidden)
                            div.chat-header__title-edit id="chat-title-edit" style="display: none;" {
                                input.chat-header__title-input id="chat-title-input" type="text" name="thread_title" value=(current_thread_title);

                                button.chat-header__title-btn.chat-header__title-btn--confirm
                                    id="chat-title-confirm"
                                    hx-post="/api/chat/title"
                                    hx-include="#chat-title-input"
                                    hx-swap="none"{
                                    (svg::check(16, 16))
                                }

                                button.chat-header__title-btn.chat-header__title-btn--cancel
                                    id="chat-title-cancel" {
                                    (svg::x(16, 16))
                                }
                            }
                        }

                        div.chat-header__right {
                            div.user-dropdown {
                                button.chat-header__user-btn id="user-menu-btn" {
                                    span { (user.name) }
                                }

                                div.dropdown-content id="user-dropdown" {
                                    div.dropdown-inner {
                                        div.dropdown-label { (user.email) }
                                        div.dropdown-separator {}

                                        div.dropdown-item.dropdown-item--disabled {
                                            div.dropdown-item__label { "Tokens Used" }
                                            div.dropdown-item__value { "12,450" }
                                        }

                                        div.dropdown-separator {}

                                        a.dropdown-item href="/api/chat/sign-out" {
                                            span { "Sign out" }
                                        }
                                    }
                                }
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
                                            (svg::thumbs_up(16, 16))
                                        }
                                        button.message__feedback-btn {
                                            (svg::thumbs_down(16, 16))
                                        }
                                    }
                                }
                            }

                            // // User message
                            // (render_user_message(
                            //     &UserMessage::new("I need help with my project."),
                            //     Utc::now(),
                            //     &http::extract_locale(&cookie_jar),
                            //     &http::extract_timezone(&cookie_jar),
                            // ))

                            // System message
                            div.message.message--system {
                                div.message__wrapper {
                                    div.message__bubble.message__bubble--system {
                                        p { "I'd be happy to help! Could you tell me more about your project and what specific assistance you need?" }
                                    }
                                    div.message__meta {
                                        span.message__time { "2:31 PM" }
                                        button.message__feedback-btn {
                                            (svg::thumbs_up(16, 16))
                                        }
                                        button.message__feedback-btn {
                                            (svg::thumbs_down(16, 16))
                                        }
                                    }
                                }
                            }
                            (render_message(
                                &Message::new(crate::thread::ThreadId::new(), Payload::SystemMessage { content: "I'd be happy to help! Could you tell me more about your project and what specific assistance you need?".into(), feedback: Some(Feedback::ThumbsUp) }),
                                &internationalization,
                            ))
                        }
                    }

                    // Input area
                    div.chat-input {
                        div.chat-input__inner {
                            div.settings-popover {
                                button.chat-input__settings-btn id="settings-btn" {
                                    (svg::settings(20, 20))
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

                            textarea.chat-input__textarea
                                id="message-input"
                                placeholder="Type your message..."
                                rows="1"
                                name="content"
                                hx-post="/api/chat/send"
                                hx-target=".chat-messages__inner"
                                hx-swap="beforeend"
                                hx-trigger="keydown[key=='Enter' && !shiftKey]" {}

                            button.button.button--primary.chat-input__send-btn
                                id="send-btn"
                                disabled
                                hx-post="/api/chat/send"
                                hx-target=".chat-messages__inner"
                                hx-include="#message-input"
                                hx-swap="beforeend" {
                                (svg::arrow_right(20, 20, 3))
                            }
                        }
                    }
                }
            }
            (super::scripts::chat_script())
        },
    ).into_response())
}

pub(crate) fn api() -> Router {
    Router::new().nest(
        PATH,
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
                    #[derive(Debug, Validate, Deserialize)]
                    struct MessageQuery {
                        // must match the "name" attribute
                        #[garde(dive)]
                        content: UserMessageContent,
                    }

                    async |internationalization: Internationalization, CurrUserId(user_id): CurrUserId,
                            CurrThreadId(thread_id): CurrThreadId,
                           Garde(Form(MessageQuery { content })): Garde<Form<MessageQuery>>| {
                        let user_id = match user_id {
                            Some(user_id) => user_id,
                            None => {
                                return Ok((StatusCode::BAD_REQUEST, "No user ID found")
                                    .into_response());
                            }
                        };
                        let thread_id = match thread_id {
                            Some(thread_id) => thread_id,
                            None => {
                                let thread = Thread::new(user_id, ThreadTitle::new_chat_title());
                                let thread_id = thread.id;
                                thread.save().await?;
                                thread_id
                            }
                        };
                        let message = Message::new(thread_id,

                            Payload::UserMessage { content });


                        AppResult::Ok(render_message(
                            &message,
                            &internationalization,
                        )
                        .into_response())
                    }
                }),
            )
            .route(
                "/title",
                post({
                    #[derive(Debug, Deserialize, Validate)]
                    struct TitleForm {
                        #[garde(dive)]
                        thread_title: ThreadTitle,
                    }

                    async |CurrThreadId(thread_id): CurrThreadId,
                           Garde(Form(TitleForm {
                               thread_title,
                           })): Garde<Form<TitleForm>>| {
                        let thread_id = match thread_id {
                            Some(thread_id) => thread_id,
                            None => {
                                // thread doesn't exist (yet)
                               return StatusCode::OK;
                            }
                        };

                        match Thread::update_title(thread_id, thread_title.clone()).await {
                            Ok(_) => {
                                debug!(%thread_id, "thread title updated");
                                StatusCode::OK
                            },
                            Err(error) => {
                                error!(?error, %thread_id, "failed to update thread title");
                                StatusCode::INTERNAL_SERVER_ERROR
                            }
                        }
                    }
                }),
            )
            .route(
                "/sign-out",
                get({
                    async |cookie_jar: CookieJar| {
                        (
                            crate::auth::remove_auth_cookie(cookie_jar),
                            Redirect::to(crate::pages::signin::PATH),
                        )
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

fn render_message(message: &Message, internationalization: &Internationalization) -> Markup {
    match &message.payload {
        Payload::UserMessage { content } => {
            // Note the user message needs to be escaped; htmx escapes by default.
            html! {
                div.message.message--user {
                    div.message__wrapper {
                        div.message__bubble.message__bubble--user {
                            p { (content) }
                        }
                        div.message__meta.message__meta--user {
                            span.message__time { (crate::datetime::today_implied_human_datetime(&message.created_at, &internationalization)) }
                            button.message__edit-btn {
                                (svg::edit(16, 16))
                            }
                        }
                    }
                }
            }
        }
        Payload::SystemMessage { content, feedback } => {
            html! {
                div.message.message--system {
                    div.message__wrapper {
                        div.message__bubble.message__bubble--system {
                            p { (content) }
                        }
                        div.message__meta {
                            span.message__time { (crate::datetime::today_implied_human_datetime(&message.created_at, &internationalization)) }
                            button class=(feedback.map(|f| if matches!(f, Feedback::ThumbsUp) { "message__feedback-btn active" } else { "" }).unwrap_or_default()) {
                                (svg::thumbs_up(16, 16))
                            }
                            button class=(feedback.map(|f| if matches!(f, Feedback::ThumbsDown) { "message__feedback-btn active" } else { "" }).unwrap_or_default()) {
                                (svg::thumbs_down(16, 16))
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extract the current thread ID from the cookies.
#[derive(Debug)]
struct CurrThreadId(Option<ThreadId>);

impl FromRequestParts<()> for CurrThreadId {
    type Rejection = Infallible;

    // Required method
    async fn from_request_parts(parts: &mut Parts, _: &()) -> Result<Self, Self::Rejection> {
        let cookie_jar = CookieJar::from_headers(&parts.headers);
        Ok(Self(extract("thread_id", &cookie_jar)))
    }
}

#[derive(Debug)]
struct CurrUserId(Option<UserId>);

impl FromRequestParts<()> for CurrUserId {
    type Rejection = Infallible;

    // Required method
    async fn from_request_parts(parts: &mut Parts, _: &()) -> Result<Self, Self::Rejection> {
        let cookie_jar = CookieJar::from_headers(&parts.headers);
        Ok(Self(extract("user_id", &cookie_jar)))
    }
}
