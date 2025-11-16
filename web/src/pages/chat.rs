use {
    crate::{
        auth::AuthUser,
        error::AppResult,
        hash::GlassVault,
        internationalization::Internationalization,
        message::{
            Feedback, Message, MessageId, Payload, SystemMessageContent, UserMessageContent,
        },
        svg,
        thread::{Thread, ThreadId, ThreadTitle},
        user::UserId,
    },
    anyhow::Context,
    axum::{
        Form, Router,
        extract::{Path, Query},
        http::StatusCode,
        response::{
            IntoResponse, Redirect,
            sse::{Event, KeepAlive, Sse},
        },
        routing::{get, post},
    },
    axum_extra::extract::CookieJar,
    axum_valid::Garde,
    chrono::{DateTime, Utc},
    futures::stream::Stream,
    garde::Validate,
    language_model::models::{LANGUAGE_MODEL_0_INFO, LANGUAGE_MODEL_1_INFO, ModelInfo},
    maud::{Markup, html},
    nonempty::NonEmpty,
    serde::Deserialize,
    std::{cmp::Reverse, convert::Infallible},
    strum::{Display, EnumIter, IntoEnumIterator},
    thousands::Separable,
    tokio_stream::StreamExt,
    tracing::*,
};

pub(crate) const PATH: &str = "/chat";

const END_OF_TRANSMISSION: &str = "\u{4}";

#[instrument(skip_all)]
pub(crate) async fn page(
    internationalization: Internationalization,
    AuthUser(user): AuthUser,
) -> AppResult<impl IntoResponse> {
    let mut threads = get_or_create_thread_items(user.id).await?;

    threads.first_mut().is_active = true;

    // the messages for the first thread
    let messages = {
        let thread = threads.first_mut();
        let mut messages = Message::get_all_messages(thread.id).await?;
        messages.sort_unstable_by_key(|m| m.created_at);

        if messages.is_empty() {
            let content = SystemMessageContent::greeting();
            let message = Message::new(
                thread.id,
                Payload::System {
                    content: content.clone(),
                    feedback: None,
                },
            );
            message.clone().save().await?;
            messages.push(message);
            thread.preview = content.to_string();
        }

        messages
    };

    let current_thread_title = threads.first().title.as_str();

    Ok(super::page(
        "Chat",
        html! {
            div.chat-container {
                // Sidebar backdrop (mobile only)
                div.chat-sidebar-backdrop id="sidebar-backdrop" {}

                // Sidebar
                aside.chat-sidebar id="chat-sidebar" {
                    (render_current_thread_id_input(Some(threads.first().id), false)?)

                    div.chat-sidebar__header {
                        button.button.button--primary.chat-sidebar__new-btn
                            hx-post="/api/chat/new"
                            hx-target=".chat-sidebar__list"
                            hx-swap="afterbegin"
                            hx-on::before-request="\
                                document.querySelector('#current-chat-item-preview')?.removeAttribute('id'); \
                                document.querySelector('.chat-item.chat-item--active')?.classList.remove('chat-item--active');" {
                            (svg::plus(16, 16))
                            span { (ThreadTitle::new_chat_title()) }
                        }
                        button.chat-sidebar__close-btn id="sidebar-close-btn" {
                            (svg::x(20, 20))
                        }
                    }

                    (render_threads(threads.iter(), user.id, &internationalization)?)

                    div.chat-sidebar__footer {
                        a.chat-sidebar__link href="/about" {
                            (svg::info_circle("icon icon--xs", 12, 12))
                            span { "About" }
                        }
                    }
                }

                // Delete confirmation modal
                div.modal-backdrop id="modal-backdrop" {}
                div.delete-confirmation-modal id="delete-confirmation-modal" {
                    h2.modal__title { "Delete Chat?" }
                    p.modal__message { "This will permanently delete this chat and all its messages." }

                    // value set by onclick handler when the inline delete button is pressed in the sidebar
                    // (the one that opens the delete confirmation modal)
                    input type="hidden" id="thread-to-delete" name="thread_id_to_delete";

                    div.modal__actions {
                        button.modal__button.modal__button--cancel id="cancel-delete-btn" {
                            "Cancel"
                        }
                        button.modal__button.modal__button--confirm
                            id="confirm-delete-btn"
                            hx-post="/api/chat/delete"
                            hx-include="#thread-to-delete, #current-thread-id"
                            hx-target=".chat-sidebar__list"
                            hx-swap="outerHTML" {
                            "Delete"
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
                        div.chat-messages__inner id="chat-messages" {
                            @for message in messages {
                                (render_message(&message, &internationalization)?)
                            }
                            // Thinking spinner (hidden by default)
                            div.message.message--system style="display: none;" {
                                div.message__wrapper {
                                    div.message__bubble.message__bubble--system {
                                        span.spinner-beachball {}
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
                                autofocus {}

                            button.button.button--primary.chat-input__send-btn
                                id="send-btn"
                                disabled
                                hx-post="/api/chat/send"
                                hx-target="#chat-messages"
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

fn render_current_thread_id_input(
    current_thread_id: Option<ThreadId>,
    out_of_band: bool,
) -> AppResult<Markup> {
    Ok(html! {
        input
            id="current-thread-id"
            type="hidden"
            name="current_thread_id"
            hx-swap-oob=[(if out_of_band { Some("true") } else { None })]
            value=(if let Some(id) = current_thread_id { GlassVault::new(id)?.to_string() } else { "".to_string() });
    })
}

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
            preview: preview.chars().take(64).collect::<String>(),
            is_active: false,
        }
    }
}

/// Get all threads for a user and return them as a vector of `ThreadItem`s
/// sorted by reverse creation date.
///
/// None of them are active.
///
/// If there are no
/// threads, a new thread is created.
#[instrument]
async fn get_or_create_thread_items(user_id: UserId) -> AppResult<NonEmpty<ThreadItem>> {
    let mut threads =
        futures::future::try_join_all(Thread::get_all(user_id).await?.into_iter().map(
            |thread| async {
                let mut messages = Message::get_all_messages(thread.id).await?;
                messages.sort_unstable_by_key(|m| Reverse(m.created_at));

                AppResult::Ok(ThreadItem::from_thread(
                    thread,
                    // if there are no messages, then the preview is the empty string
                    messages
                        .first()
                        .map(|m| m.payload.as_str())
                        .unwrap_or_default(),
                ))
            },
        ))
        .await?;

    threads.sort_unstable_by_key(|t| Reverse(t.created_at));

    let threads = match NonEmpty::from_vec(threads) {
        Some(threads) => threads,
        None => {
            let thread = Thread::new(user_id, ThreadTitle::new_chat_title());
            thread.clone().save().await?;
            let content = SystemMessageContent::greeting();
            let message = Message::new(
                thread.id,
                Payload::System {
                    content: content.clone(),
                    feedback: None,
                },
            );
            message.save().await?;
            NonEmpty::new(ThreadItem::from_thread(thread, content.as_str()))
        }
    };

    Ok(threads)
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
    current_thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn send_message(
    internationalization: Internationalization,
    Garde(Form(SendForm {
        content,
        current_thread_id,
    })): Garde<Form<SendForm>>,
) -> AppResult<impl IntoResponse> {
    let thread_id = current_thread_id.into_inner();
    let message = Message::new(thread_id, Payload::User { content });
    message.clone().save().await?;

    // TODO: find the user's queue, and add thread_id to it
    let partial_message = Message::new(
        thread_id,
        Payload::PartialSystem {
            content: SystemMessageContent::new(""),
        },
    );
    partial_message.clone().save().await?;

    Ok(html! {
        (render_message(&message, &internationalization)?)
        (render_message(&partial_message, &internationalization)?)
        // update the preview in the sidebar
        div hx-swap-oob="innerHTML:#current-chat-item-preview" {
            (message.payload.as_str().trim().chars().take(10).collect::<String>())
        }
    }
    .into_response())
}

#[derive(Debug, Deserialize, Validate)]
struct TitleForm {
    #[garde(dive)]
    title: ThreadTitle,
    #[garde(skip)]
    current_thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn update_title(
    Garde(Form(TitleForm {
        title,
        current_thread_id,
    })): Garde<Form<TitleForm>>,
) -> AppResult<String> {
    let current_thread_id = current_thread_id.into_inner();
    let mut thread = Thread::by_id(current_thread_id)
        .await?
        .context("thread not found")?;
    thread.title = title.clone();
    thread.save().await?;
    debug!(%current_thread_id, %title, "thread title updated");
    Ok(title.to_string())
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
    Ok(render_feedback_form(message_id, Some(feedback)).into_response())
}

#[derive(Debug, Deserialize, Validate)]
struct UpdateMessageForm {
    #[garde(dive)]
    content: UserMessageContent,
    #[garde(skip)]
    message_id: GlassVault<MessageId>,
}

#[instrument]
async fn update_message(
    internationalization: Internationalization,
    Garde(Form(UpdateMessageForm {
        content,
        message_id,
    })): Garde<Form<UpdateMessageForm>>,
) -> AppResult<impl IntoResponse> {
    let message_id = message_id.into_inner();
    debug!(%message_id, "updating message content");
    let mut message = Message::by_id(message_id).await?;
    message.payload = Payload::User { content };
    let response = render_message(&message, &internationalization)?.into_response();
    message.save().await?;
    Ok(response)
}

#[instrument]
async fn new_thread(
    internationalization: Internationalization,
    AuthUser(user): AuthUser,
) -> AppResult<Markup> {
    let thread = Thread::new(user.id, ThreadTitle::new_chat_title());
    thread.clone().save().await?;

    let content = SystemMessageContent::greeting();
    let message = Message::new(
        thread.id,
        Payload::System {
            content: content.clone(),
            feedback: None,
        },
    );
    message.save().await?;

    let mut thread = ThreadItem::from_thread(thread, content.as_str());
    thread.is_active = true;

    Ok(html! {
        (render_thread_item(&thread, user.id, &internationalization)?)
        (render_current_thread_id_input(Some(thread.id), true)?)
    })
}

fn render_threads<'a>(
    threads: impl Iterator<Item = &'a ThreadItem>,
    user_id: UserId,
    internationalization: &Internationalization,
) -> AppResult<Markup> {
    Ok(html! {
         div.chat-sidebar__list {
            @for thread in threads {
                (render_thread_item(thread, user_id, internationalization)?)
            }
        }
    })
}

#[derive(Debug, Deserialize)]
struct SelectForm {
    thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn select_thread(
    internationalization: Internationalization,
    AuthUser(user): AuthUser,
    Form(SelectForm { thread_id }): Form<SelectForm>,
) -> AppResult<impl IntoResponse> {
    let thread_id = thread_id.into_inner();
    let mut threads = get_or_create_thread_items(user.id).await?;

    let title = match threads.iter_mut().find(|thread| thread.id == thread_id) {
        Some(thread) => {
            thread.is_active = true;
            thread.title.clone()
        }
        None => {
            return Ok((StatusCode::NOT_FOUND, "Thread not found").into_response());
        }
    };

    let mut messages = Message::get_all_messages(thread_id).await?;
    messages.sort_unstable_by_key(|m| m.created_at);

    Ok(html! {
        (render_threads(threads.iter(), user.id, &internationalization)?)
        (render_current_thread_id_input(Some(thread_id), true)?)
        div hx-swap-oob="innerHTML:#chat-messages" {
            @for message in messages {
                (render_message(&message, &internationalization)?)
            }
        }
        div hx-swap-oob="innerHTML:#chat-title-display" { (title) }
        script { "document.getElementById('message-input')?.focus();" }
    }
    .into_response())
}

#[derive(Debug, Deserialize)]
struct DeleteForm {
    thread_id_to_delete: GlassVault<ThreadId>,
    current_thread_id: GlassVault<ThreadId>,
}

#[instrument]
async fn delete_thread(
    internationalization: Internationalization,
    AuthUser(user): AuthUser,
    Form(DeleteForm {
        thread_id_to_delete,
        current_thread_id,
    }): Form<DeleteForm>,
) -> AppResult<impl IntoResponse> {
    let thread_id_to_delete = thread_id_to_delete.into_inner();
    let current_thread_id = current_thread_id.into_inner();

    // index of the  thread to delete
    let index = {
        let mut threads = Thread::get_all(user.id).await?;
        threads.sort_unstable_by_key(|t| Reverse(t.created_at));
        threads
            .iter()
            .position(|t| t.id == thread_id_to_delete)
            .context("thread not found")?
    };

    Thread::delete(thread_id_to_delete).await?;
    debug!(%thread_id_to_delete, "thread deleted");

    if thread_id_to_delete == current_thread_id {
        let mut threads = get_or_create_thread_items(user.id).await?;

        let next_index = index.saturating_add(1);
        let prev_index = index.saturating_sub(1);
        let thread = threads
            .get_mut(
                threads
                    .get(next_index)
                    .map(|_| next_index)
                    .or_else(|| threads.get(prev_index).map(|_| prev_index))
                    .unwrap_or_default(),
            )
            .context("thread not found")?;

        thread.is_active = true;

        // Get messages for the newly selected thread
        let messages = {
            let mut messages = Message::get_all_messages(thread.id).await?;
            messages.sort_unstable_by_key(|m| m.created_at);

            let content = SystemMessageContent::greeting();
            if messages.is_empty() {
                let message = Message::new(
                    thread.id,
                    Payload::System {
                        content: content.clone(),
                        feedback: None,
                    },
                );
                message.clone().save().await?;
                messages.push(message);
            }
            messages
        };

        // Return updated UI with new thread list, messages, and current thread ID
        Ok(html! {
            (render_threads(threads.iter(), user.id, &internationalization)?)
            (render_current_thread_id_input(Some(threads.first().id), true)?)
            div hx-swap-oob="innerHTML:#chat-messages" {
                @for message in messages {
                    (render_message(&message, &internationalization)?)
                }
            }
            div hx-swap-oob="innerHTML:#chat-title-display" {
                (threads.first().title.as_str())
            }
        }
        .into_response())
    } else {
        // if the deleted thread was not the current one, just return the updated thread
        // list
        let mut threads = get_or_create_thread_items(user.id).await?;

        // need to find and mark the current thread as active again
        if let Some(thread) = threads.iter_mut().find(|t| t.id == current_thread_id) {
            thread.is_active = true;
        }

        Ok(render_threads(threads.iter(), user.id, &internationalization)?.into_response())
    }
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
        Payload::User { content } => {
            let message_id_str = format!("message-{}", message.id);
            let message_display_id = format!("{message_id_str}-display");
            let message_edit_id = format!("{message_id_str}-edit");
            let message_input_id = format!("{message_id_str}-input");
            let message_meta_display_id = format!("{message_id_str}-meta-display");
            let message_meta_edit_id = format!("{message_id_str}-meta-edit");

            // Note the user message needs to be escaped; htmx escapes by default.
            html! {
                div.message.message--user id=(message_id_str) {
                    input type="hidden" name="message_id" value=(GlassVault::new(message.id)?);
                    div.message__wrapper {
                        // Display mode
                        div.message__bubble.message__bubble--user id=(message_display_id) {
                            (content)
                        }

                        // Edit mode (initially hidden)
                        div.message__bubble.message__bubble--user.message__bubble--edit id=(message_edit_id) style="display: none;" {
                            textarea.message__edit-input
                                id=(message_input_id)
                                name="content"
                                rows="1"
                                maxlength="1024" {
                                (content)
                            }
                        }

                        // Meta - Display mode (with edit button)
                        div.message__meta id=(message_meta_display_id) {
                            span.message_subdued { (crate::datetime::today_implied_readable_datetime(&message.created_at, &internationalization)) }
                            button.message__edit-btn {
                                (svg::edit(16, 16))
                            }
                        }

                        // Meta - Edit mode (with confirm/cancel buttons, initially hidden)
                        div.message__meta id=(message_meta_edit_id) style="display: none;" {
                            span.message_subdued { (crate::datetime::today_implied_readable_datetime(&message.created_at, &internationalization)) }
                            button.message__edit-confirm
                                type="button"
                                hx-post="/api/chat/update"
                                hx-include=(format!("#{}, #{}", message_input_id, format!("message-{}-hidden-id", message.id)))
                                hx-target=(format!("#{}", message_id_str))
                                hx-swap="outerHTML" {
                                (svg::check(16, 16))
                            }
                            button.message__edit-cancel
                                type="button" {
                                (svg::x(16, 16))
                            }
                        }
                    }
                    input type="hidden" id=(format!("message-{}-hidden-id", message.id)) name="message_id" value=(GlassVault::new(message.id)?);
                }
            }
        }
        Payload::System { content, feedback } => {
            html! {
                div.message.message--system {
                    div.message__wrapper {
                        div.message__bubble.message__bubble--system {
                            (content)
                        }
                        div.message__meta {
                            span.message_subdued { (crate::datetime::today_implied_readable_datetime(&message.created_at, &internationalization)) }
                            (render_feedback_form(message.id, *feedback)?)
                        }
                    }
                }
            }
        }
        Payload::PartialSystem { content } => {
            html! {
                div.message.message--system hx-ext="sse" sse-connect=(format!("/api/chat/response?message_id={}", GlassVault::new(message.id)?)) {
                    div.message__wrapper {
                        div.message__bubble.message__bubble--system sse-swap="Content" {
                            @if content.is_empty() {
                                span.spinner-beachball {}
                            } @else {
                                (content)
                            }
                        }
                        div.message__meta {
                            span.message_subdued.shimmer-text id="system-state" {
                                span sse-swap="SystemState" { ("Waiting for GPU to become available...") }
                            }
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
    user_id: UserId,
    internationalization: &Internationalization,
) -> AppResult<Markup> {
    let thread_id = GlassVault::new(thread.id)?;
    Ok(html! {
        div class=(if thread.is_active { "chat-item chat-item--active" } else { "chat-item" }) {
            div.chat-item__content
                hx-get="/api/chat/select"
                hx-vals=(format!(r#"{{"thread_id": "{}", "user_id": "{}"}}"#, thread_id, user_id))
                hx-target="div.chat-sidebar__list"
                hx-swap="outerHTML"
                hx-on::before-request="document.getElementById('chat-title-edit').style.display = 'none';" {
                div.chat-item__header {
                    (svg::chat_bubble(16, 16))
                    span.chat-item__title id=(if thread.is_active { "chat-item-title" } else { "" }) { (thread.title) }
                    span.chat-item__time { (crate::datetime::today_implied_readable_datetime(&thread.created_at, &internationalization)) }
                    button.chat-item__delete-btn
                        type="button"
                        data-thread-id=(thread_id)
                        onclick="\
                            document.getElementById('thread-to-delete').value = this.dataset.threadId; \
                            document.getElementById('modal-backdrop').classList.add('is-visible'); \
                            document.getElementById('delete-confirmation-modal').classList.add('is-visible');" {
                        (svg::x(14, 14))
                    }
                }
                p.chat-item__preview id=(if thread.is_active { "current-chat-item-preview" } else { "" }) { (thread.preview) }
            }
        }
    })
}

const has_send: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Deserialize)]
struct StreamQuery {
    message_id: GlassVault<MessageId>,
}

async fn stream_response(
    Query(StreamQuery { message_id }): Query<StreamQuery>,
) -> impl IntoResponse {
    let message_id = message_id.into_inner();
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let mut stream =
        tokio_stream::wrappers::ReceiverStream::new(rx).map(Result::<_, Infallible>::Ok);

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut message = String::from("The ");

        let tx_clone = tx.clone();

        tokio::spawn(async move {
            loop {
                tx_clone
                    .send(
                        Event::default()
                            .event("SystemState")
                            .data("Generating response..."),
                    )
                    .await
                    .unwrap();

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let html = html! {
                p {
                    (message)
                    // span.spinner-beachball {}
                }
            };
            tx.send(Event::default().event("Content").data(html.into_string()))
                .await
                .unwrap();

            message += " word";
        }
    });

    // TODO: it should be 1 SSE connetion per user client!

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
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
            .route("/update", post(update_message))
            .route("/new", post(new_thread))
            .route("/select", get(select_thread))
            .route("/delete", post(delete_thread))
            .route("/response", get(stream_response)),
    )
}
