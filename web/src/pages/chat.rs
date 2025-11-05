use {
    crate::{
        auth::require_auth_user,
        error::{AppResult, ResponseResult},
        hash::GlassVault,
        internationalization::Internationalization,
        message::{
            Feedback, Message, MessageId, Payload, SystemMessageContent, UserMessageContent,
        },
        svg,
        thread::{Thread, ThreadId, ThreadTitle},
    },
    axum::{
        Form, Router,
        extract::Query,
        http::StatusCode,
        response::{IntoResponse, Redirect},
        routing::{get, post},
    },
    axum_extra::extract::CookieJar,
    axum_valid::Garde,
    chrono::{DateTime, Utc},
    garde::Validate,
    language_model::models::{LANGUAGE_MODEL_0_INFO, LANGUAGE_MODEL_1_INFO, ModelInfo},
    maud::{Markup, html},
    serde::Deserialize,
    std::cmp::Reverse,
    strum::{Display, EnumIter, IntoEnumIterator},
    thousands::Separable,
    tracing::*,
};

pub(crate) const PATH: &str = "/chat";

#[derive(Debug)]
struct ThreadItem {
    id: ThreadId,
    title: ThreadTitle,
    created_at: DateTime<Utc>,
    preview: String,
    is_active: bool,
}

impl ThreadItem {
    fn from_thread(thread: Thread, preview: &str) -> ThreadItem {
        let preview = preview.chars().take(100).collect::<String>();
        ThreadItem {
            id: thread.id,
            title: thread.title,
            created_at: thread.created_at,
            preview: preview.chars().take(32).collect::<String>(),
            is_active: false,
        }
    }
}

pub(crate) async fn page(
    internationalization: Internationalization,
    cookie_jar: CookieJar,
) -> ResponseResult {
    tracing::info!("chat page requested");
    let user = require_auth_user(&cookie_jar).await?;

    let mut threads = {
        let mut threads = Thread::get_all(user.id).await?;

        threads.sort_unstable_by_key(|t| Reverse(t.created_at));

        if threads.is_empty() {
            let thread = Thread::new(user.id, ThreadTitle::new_chat_title());
            thread.clone().save().await?;
            threads.push(thread);
        }

        let mut thread_items = Vec::with_capacity(threads.len());
        for thread in threads {
            let mut messages = Message::get_all_messages(thread.id).await?;
            messages.sort_unstable_by_key(|m| Reverse(m.created_at));
            let preview = messages
                .first()
                .map(|m| match &m.payload {
                    Payload::UserMessage { content } => content.as_str(),
                    Payload::SystemMessage { content, .. } => content.as_str(),
                })
                .unwrap_or_default();

            thread_items.push(ThreadItem::from_thread(thread, preview));
        }

        if let Some(thread) = thread_items.first_mut() {
            thread.is_active = true;
        }

        thread_items
    };

    // the messages for the first thread
    let messages = {
        if let Some(thread) = threads.first_mut() {
            let mut messages = Message::get_all_messages(thread.id).await?;
            messages.sort_unstable_by_key(|m| m.created_at);

            let content = SystemMessageContent::greeting();
            if messages.is_empty() {
                let message = Message::new(
                    thread.id,
                    Payload::SystemMessage {
                        content: content.clone(),
                        feedback: None,
                    },
                );
                message.clone().save().await?;
                messages.push(message);
                thread.preview = content.to_string();
            }

            messages
        } else {
            // does not allocate
            Vec::new()
        }
    };

    let current_thread_title = threads
        .first()
        .map(|thread| thread.title.as_str())
        .unwrap_or_default();

    Ok(super::page(
        "Chat",
        html! {
            div.chat-container {
                // Sidebar backdrop (mobile only)
                div.chat-sidebar-backdrop id="sidebar-backdrop" {}

                // Sidebar
                aside.chat-sidebar id="chat-sidebar" {
                    input
                        id="current-thread-id"
                        type="hidden"
                        name="thread_id"
                        value=(if let Some(thread) = threads.first() { GlassVault::new(thread.id)?.to_string() } else { "".to_string() });

                    div.chat-sidebar__header {
                        button.button.button--primary.chat-sidebar__new-btn
                            hx-post="/api/chat/new"
                            hx-target=".chat-sidebar__list"
                            hx-swap="afterbegin"
                            hx-on::before-request="\
                                document.querySelector('#chat-item-preview')?.removeAttribute('id'); \
                                document.querySelector('.chat-item.chat-item--active')?.classList.remove('chat-item--active');" {
                            (svg::plus(16, 16))
                            span { (ThreadTitle::new_chat_title()) }
                        }
                        button.chat-sidebar__close-btn id="sidebar-close-btn" {
                            (svg::x(20, 20))
                        }
                    }

                    div.chat-sidebar__list {
                        @for thread in &threads {
                            (render_thread_item(thread, &internationalization)?)
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
                            button.chat-header__menu-btn id="sidebar-menu-btn" {
                                (svg::menu(24, 24))
                            }
                            div.chat-header__logo {
                                span { "AI" }
                            }
                            // Title display mode
                            h1.chat-header__title id="chat-title-display" { (current_thread_title) }

                            // Title edit mode (initially hidden)
                            div.chat-header__title-edit id="chat-title-edit" style="display: none;" {
                                input.chat-header__title-input
                                    id="chat-title-input"
                                    type="text"
                                    name="title"
                                    maxlength="32"
                                    autocapitalize="words"
                                    value=(current_thread_title);

                                button.chat-header__title-btn.chat-header__title-btn--confirm
                                    id="chat-title-confirm"
                                    hx-post="/api/chat/title"
                                    hx-include="#chat-title-input, #current-thread-id"
                                    hx-target="#chat-item-title" {
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
                                    (svg::user("icon", 20, 20))
                                    span.chat-header__user-name { (user.name) }
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
                            @for message in messages {
                                (render_message(&message, &internationalization)?)
                            }
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
                                name="content" {}

                            button.button.button--primary.chat-input__send-btn
                                id="send-btn"
                                disabled
                                hx-post="/api/chat/send"
                                hx-target=".chat-messages__inner"
                                hx-include="#message-input, #current-thread-id"
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

#[derive(Debug, Deserialize)]
struct ModelQuery {
    // must match the "name" attribute
    model: ModelSelection,
}

#[instrument]
async fn get_models(Query(ModelQuery { model }): Query<ModelQuery>) -> impl IntoResponse {
    render_model_details(model).into_response()
}

#[derive(Debug, Validate, Deserialize)]
struct SendForm {
    #[garde(dive)]
    content: UserMessageContent,
    #[garde(skip)]
    thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn send_message(
    internationalization: Internationalization,
    Garde(Form(SendForm { content, thread_id })): Garde<Form<SendForm>>,
) -> AppResult<impl IntoResponse> {
    let message = Message::new(thread_id.into_inner(), Payload::UserMessage { content });
    message.clone().save().await?;

    AppResult::Ok(
        html! {
            (render_message(&message, &internationalization)?)
            div.chat-item__preview id="chat-item-preview" hx-swap-oob="true" {
                (match &message.payload {
                    Payload::UserMessage { content } => content.to_string(),
                    Payload::SystemMessage { content, .. } => content.to_string(),
                }.trim().chars().take(10).collect::<String>())
            }
        }
        .into_response(),
    )
}

#[derive(Debug, Deserialize, Validate)]
struct TitleForm {
    #[garde(dive)]
    title: ThreadTitle,
    #[garde(skip)]
    thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn update_title(
    Garde(Form(TitleForm { title, thread_id })): Garde<Form<TitleForm>>,
) -> AppResult<String> {
    let thread_id = thread_id.into_inner();
    Thread::update_title(thread_id, title.clone()).await?;
    debug!(%thread_id, %title, "thread title updated");
    AppResult::Ok(title.to_string())
}

#[instrument]
async fn sign_out(cookie_jar: CookieJar) -> impl IntoResponse {
    (
        crate::auth::remove_auth_cookie(cookie_jar),
        Redirect::to(crate::pages::signin::PATH),
    )
}

#[derive(Debug, Deserialize)]
struct FeedbackForm {
    message_id: GlassVault<MessageId>,
    feedback: Feedback,
}

#[instrument]
async fn update_feedback(
    Form(FeedbackForm {
        message_id,
        feedback,
    }): Form<FeedbackForm>,
) -> AppResult<impl IntoResponse> {
    let message_id = message_id.into_inner();
    debug!(%message_id, ?feedback, "updating message feedback");
    Message::update_feedback(message_id, feedback).await?;
    AppResult::Ok(render_feedback_form(message_id, Some(feedback)).into_response())
}

#[instrument]
async fn new_thread(
    internationalization: Internationalization,
    cookie_jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    let user = require_auth_user(&cookie_jar).await?;
    let thread = Thread::new(user.id, ThreadTitle::new_chat_title());
    thread.clone().save().await?;

    let content = SystemMessageContent::greeting();
    let message = Message::new(
        thread.id,
        Payload::SystemMessage {
            content: content.clone(),
            feedback: None,
        },
    );
    message.save().await?;

    let mut thread = ThreadItem::from_thread(thread, content.as_str());
    thread.is_active = true;

    AppResult::Ok(
        html! {
            (render_thread_item(&thread, &internationalization)?)
            input id="current-thread-id"
            type="hidden"
            name="thread_id"
            hx-swap-oob="true"
            value=(GlassVault::new(thread.id)?);
        }
        .into_response(),
    )
}

#[derive(Debug, Deserialize)]
struct SelectForm {
    thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn select_thread(
    _internationalization: Internationalization,
    Form(SelectForm { thread_id }): Form<SelectForm>,
) -> AppResult<String> {
    let thread_id = thread_id.into_inner();
    let _messages = Message::get_all_messages(thread_id).await?;

    AppResult::Ok("Hello, world!".to_string())
    // AppResult::Ok(html! {
    //     (render_thread_item(&thread, &internationalization)?)
    //     input id="current-thread-id"
    //     type="hidden" name="thread_id"
    //      hx-swap-oob="true" value=(GlassVault::new(thread.id)?);
    // }.into_response())
}

#[derive(Debug, Deserialize)]
struct DeleteForm {
    thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn delete_thread(Form(DeleteForm { thread_id }): Form<DeleteForm>) -> AppResult<StatusCode> {
    let thread_id = thread_id.into_inner();
    Thread::delete(thread_id).await?;
    debug!(%thread_id, "thread deleted");
    AppResult::Ok(StatusCode::OK)
}

pub(crate) fn api() -> Router {
    Router::new().nest(
        PATH,
        Router::new()
            .route("/models", get(get_models))
            .route("/send", post(send_message))
            .route("/title", post(update_title))
            .route("/sign-out", get(sign_out))
            .route("/feedback", post(update_feedback))
            .route("/new", post(new_thread))
            .route("/select", get(select_thread))
            .route("/delete", post(delete_thread)),
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

fn render_message(
    message: &Message,
    internationalization: &Internationalization,
) -> AppResult<Markup> {
    Ok(match &message.payload {
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
                            (render_feedback_form(message.id, *feedback)?)
                        }
                    }
                }
            }
        }
    })
}

fn render_feedback_form(message_id: MessageId, feedback: Option<Feedback>) -> AppResult<Markup> {
    let form_id = format!("feedback-form-{message_id}");
    Ok(html! {
        form."feedback-form" id=(form_id) {
            input type="hidden" name="message_id" value=(GlassVault::new(message_id)?);

            button class=(
                format!("message__feedback-btn{}", if matches!(feedback, Some(Feedback::ThumbsUp)) { " active" } else { "" })
            )
            type="button"
            name="feedback"
            value="ThumbsUp"
            hx-post="/api/chat/feedback"
            hx-target=(format!("#{form_id}")) {
                (svg::thumbs_up(16, 16))
            }
            button class=(
                format!("message__feedback-btn{}", if matches!(feedback, Some(Feedback::ThumbsDown)) { " active" } else { "" })
            )
            type="button"
            name="feedback"
            value="ThumbsDown"
            hx-post="/api/chat/feedback"
            hx-target=(format!("#{form_id}")) {
                (svg::thumbs_down(16, 16))
            }
        }
    })
}

fn render_thread_item(
    thread: &ThreadItem,
    internationalization: &Internationalization,
) -> AppResult<Markup> {
    Ok(html! {
        div.chat-item-swipe-wrapper data-thread-id=(GlassVault::new(thread.id)?) {
            // Delete button (hidden behind)
            div.chat-item-delete-btn {
                button.chat-item-delete-btn__button
                    hx-post="/api/chat/delete"
                    hx-vals=(format!(r#"{{"thread_id": "{}"}}"#, GlassVault::new(thread.id)?))
                    hx-target="closest .chat-item-swipe-wrapper"
                    hx-swap="outerHTML swap:300ms" {
                    (svg::x(20, 20))
                }
            }
            // Chat item (swipeable)
            div class=(if thread.is_active { "chat-item chat-item--active" } else { "chat-item" }) {
                div.chat-item__content {
                    div.chat-item__header {
                        (svg::chat_bubble(16, 16))
                        span.chat-item__title id=(if thread.is_active { "chat-item-title" } else { "" }) { (thread.title) }
                        span.chat-item__time { (crate::datetime::today_implied_human_datetime(&thread.created_at, &internationalization)) }
                    }
                    p.chat-item__preview id=(if thread.is_active { "chat-item-preview" } else { "" }) { (thread.preview) }
                }
            }
        }
    })
}
