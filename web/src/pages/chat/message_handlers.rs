use {
    super::{
        types::{FeedbackForm, SendForm, StreamQuery, UpdateMessageForm},
        views::{render_feedback_form, render_message},
    },
    crate::{
        auth::AuthUser,
        error::AppResult,
        inference::InferenceRequest,
        internationalization::Internationalization,
        message::{Message, PartialSystemMessage, Payload, SystemMessage, SystemMessageMarkdown},
        pages::chat::{sse::SystemStateLabel, views::render_messages},
        scheduler,
    },
    axum::{Form, extract::Query, response::IntoResponse},
    axum_valid::Garde,
    maud::{Markup, html},
    tracing::*,
};

pub(super) fn render_preview_oob(content: &str) -> Markup {
    html! {
        div hx-swap-oob="innerHTML:#current-chat-item-preview" {
            (content)
        }
    }
}

#[instrument]
pub(super) async fn send_message(
    AuthUser(user): AuthUser,
    internationalization: Internationalization,
    Garde(Form(SendForm {
        content,
        current_thread_id,
        model_id,
        temperature,
    })): Garde<Form<SendForm>>,
) -> AppResult<impl IntoResponse> {
    let thread_id = current_thread_id.into_inner();
    let message = Message::new(thread_id, Payload::User { content });
    message.clone().save().await?;

    scheduler::queue_task(
        user.id,
        thread_id,
        InferenceRequest {
            model_id,
            temperature,
        },
    );
    let partial_message = Message::new(
        thread_id,
        Payload::PartialSystem {
            content: SystemMessageMarkdown::new(""),
        },
    );
    partial_message.clone().save().await?;

    Ok(html! {
        (render_message(&message, &internationalization)?)
        (render_message(&partial_message, &internationalization)?)
        (render_preview_oob(message.payload.as_str()))
    }
    .into_response())
}

#[instrument]
pub(super) async fn update_message(
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
    message.clone().save().await?;

    // delete all later message in the thread
    let mut messages = Message::get_all_messages(message.thread_id).await?;
    messages.sort_unstable_by_key(|m| m.created_at);
    let delete_timestamp = message.created_at;
    for message in messages {
        if message.created_at > delete_timestamp {
            message.delete().await?;
        }
    }

    // TODO: find the user's queue, and add thread_id to it
    let partial_message = Message::new(
        message.thread_id,
        Payload::PartialSystem {
            content: SystemMessageMarkdown::new(""),
        },
    );
    partial_message.clone().save().await?;

    let mut messages = Message::get_all_messages(message.thread_id).await?;
    messages.sort_unstable_by_key(|m| m.created_at);

    Ok(html! {
        (render_messages(&messages, &internationalization)?)
        (render_preview_oob(message.payload.as_str()))
    }
    .into_response())
}

#[instrument]
pub(super) async fn update_feedback(
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

pub(super) async fn stream_response(
    internationalization: Internationalization,
    Query(StreamQuery { message_id }): Query<StreamQuery>,
) -> AppResult<impl IntoResponse> {
    let mut message = Message::by_id(message_id.into_inner()).await?;
    let (sse_tx, sse_rx) = super::sse::new_sse_channel();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut partial_system_message = PartialSystemMessage {
            id: message.id,
            thread_id: message.thread_id,
            created_at: message.created_at,
            content: SystemMessageMarkdown::new("# Dogs\nI like dogs.\nThe "),
        };

        tokio::spawn({
            let sse_tx = sse_tx.clone();
            async move {
                loop {
                    if sse_tx
                        .send_system_state(SystemStateLabel::GeneratingResponse)
                        .await
                        .is_err()
                    {
                        // stop sending system state messages when channel is closed
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        });

        for _ in 0..5 {
            partial_system_message.content += " word";

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            if let Err(error) = sse_tx
                .send_partial_system_message(&partial_system_message)
                .await
            {
                warn!(?error);
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // save the final system message
        message.payload = Payload::System {
            content: partial_system_message.content.clone(),
            feedback: None,
        };

        if let Err(error) = message.clone().save().await {
            warn!(?error, "failed to save message");
        }

        // send final message
        if let Err(error) = sse_tx
            .send_final_system_message(
                &SystemMessage::try_from(message)
                    .expect("failed to convert message to system message"),
                &internationalization,
            )
            .await
        {
            warn!(?error, "failed to send final system message");
        }
    });

    Ok(sse_rx.into_response())
}
