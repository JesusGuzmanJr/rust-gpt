use {
    super::{
        types::{DeleteForm, SelectForm, ThreadItem, TitleForm},
        views::{render_current_thread_id_input, render_message, render_thread_item, render_threads},
    },
    crate::{
        auth::AuthUser,
        error::AppResult,
        internationalization::Internationalization,
        message::{Message, Payload, SystemMessageContent},
        thread::{Thread, ThreadTitle},
        user::UserId,
    },
    anyhow::Context,
    axum::{
        http::StatusCode,
        response::IntoResponse,
        Form,
    },
    axum_valid::Garde,
    maud::{html, Markup},
    nonempty::NonEmpty,
    std::cmp::Reverse,
    tracing::*,
};

/// Get all threads for a user and return them as a vector of `ThreadItem`s
/// sorted by reverse creation date.
///
/// None of them are active.
///
/// If there are no
/// threads, a new thread is created.
#[instrument]
pub(super) async fn get_or_create_thread_items(user_id: UserId) -> AppResult<NonEmpty<ThreadItem>> {
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

#[instrument]
pub(super) async fn new_thread(
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

#[instrument]
pub(super) async fn select_thread(
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

#[instrument]
pub(super) async fn delete_thread(
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

#[instrument]
pub(super) async fn update_title(
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
