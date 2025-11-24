use {
    super::types::{END_OF_TRANSMISSION, ModelSelection, ThreadItem},
    crate::{
        auth::AuthUser,
        error::AppResult,
        hash::GlassVault,
        internationalization::Internationalization,
        message::{Feedback, Message, MessageId, Payload, SystemMessageMarkdown},
        svg,
        thread::{ThreadId, ThreadTitle},
        user::UserId,
    },
    axum::response::IntoResponse,
    language_model::models::ModelInfo,
    maud::{Markup, html},
    strum::IntoEnumIterator,
    thousands::Separable,
    tracing::*,
};

#[instrument(skip_all)]
pub(crate) async fn page(
    internationalization: Internationalization,
    AuthUser(user): AuthUser,
) -> AppResult<impl IntoResponse> {
    let mut threads = super::thread_handlers::get_or_create_thread_items(user.id).await?;

    threads.first_mut().is_active = true;

    // the messages for the first thread
    let messages = {
        let thread = threads.first_mut();
        let mut messages = Message::get_all_messages(thread.id).await?;
        messages.sort_unstable_by_key(|m| m.created_at);

        if messages.is_empty() {
            let content = SystemMessageMarkdown::greeting();
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

    Ok(super::super::page(
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
            (super::super::scripts::chat_script())
        },
    )
    .into_response())
}

pub(super) fn render_current_thread_id_input(
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

pub(super) fn render_threads<'a>(
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

pub(super) fn render_thread_item(
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
                div.chat-item__preview id=(if thread.is_active { "current-chat-item-preview" } else { "" }) { (thread.preview) }
            }
        }
    })
}

pub(super) fn render_message(
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
                            (content.to_html())
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
                div.message.message--system hx-ext="sse" sse-close=(END_OF_TRANSMISSION) sse-connect=(format!("/api/chat/response?message_id={}", GlassVault::new(message.id)?)) {
                    div.message__wrapper id="partial-system-message" {
                        div.message__bubble.message__bubble--system sse-swap="Content" {
                            @if content.is_empty() {
                                span.spinner-beachball {}
                            } @else {
                                (content.to_html())
                            }
                        }
                        div.message__meta {
                            span.message_subdued.shimmer-text id="system-state" {
                                span sse-swap="SystemState" { ("Connecting to inference server...") }
                            }
                        }
                    }
                }
            }
        }
    })
}

pub(super) fn render_feedback_form(
    message_id: MessageId,
    feedback: Option<Feedback>,
) -> AppResult<Markup> {
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

pub(super) fn render_model_details(selection: ModelSelection) -> Markup {
    let model_info: ModelInfo = selection.into();
    html! {
        div.model-detail { (format!("Corpus Size: {}", model_info.corpus_size.display().iec())) }
        div.model-detail { (format!("Vocabulary Size: {}", model_info.vocabulary_size.separate_with_commas())) }
    }
}

pub(super) fn render_messages(
    messages: &[Message],
    internationalization: &Internationalization,
) -> AppResult<Markup> {
    Ok(html! {
        div hx-swap-oob="innerHTML:#chat-messages" {
            @for message in messages {
                (render_message(message, internationalization)?)
            }
        }
    })
}
