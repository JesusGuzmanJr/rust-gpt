use {
    super::{
        types::{END_OF_TRANSMISSION, FeedbackForm, SendForm, StreamQuery, UpdateMessageForm},
        views::{render_feedback_form, render_message},
    },
    crate::{
        auth::AuthUser,
        error::AppResult,
        inference::InferenceJob,
        internationalization::Internationalization,
        message::{Message, Payload, SystemMessageMarkdown},
        pages::chat::views::render_messages,
        runner,
    },
    axum::{
        Form,
        extract::{Query, State},
        response::{
            IntoResponse,
            sse::{Event, KeepAlive, Sse},
        },
    },
    axum_valid::Garde,
    maud::{Markup, html},
    std::convert::Infallible,
    tokio_stream::StreamExt,
    tracing::*,
};

fn render_preview_oob(content: &str) -> Markup {
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
    })): Garde<Form<SendForm>>,
) -> AppResult<impl IntoResponse> {
    let thread_id = current_thread_id.into_inner();
    let message = Message::new(thread_id, Payload::User { content });
    message.clone().save().await?;

    runner::queue_task(user.id, thread_id, InferenceJob {});
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
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(Result::<_, Infallible>::Ok);

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut system_message_markdown = SystemMessageMarkdown::new("# Dogs\nI like dogs.\nThe ");

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

        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let html = html! {
                (system_message_markdown.to_html())
                (render_preview_oob(system_message_markdown.as_str()))
            };
            tx.send(Event::default().event("Content").data(html.into_string()))
                .await
                .unwrap();

            system_message_markdown += " word";
        }

        system_message_markdown += ".\n\nAnd";

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let html = html! {
            (system_message_markdown.to_html())
            (render_preview_oob(system_message_markdown.as_str()))
        };
        tx.send(Event::default().event("Content").data(html.into_string()))
            .await
            .unwrap();

        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let html = html! {
                (system_message_markdown.to_html())
                (render_preview_oob(system_message_markdown.as_str()))
            };
            tx.send(Event::default().event("Content").data(html.into_string()))
                .await
                .unwrap();

            system_message_markdown += " word";
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        message.payload = Payload::System {
            content: SystemMessageMarkdown::new(system_message_markdown.clone()),
            feedback: None,
        };
        message.clone().save().await.unwrap();

        // send final message
        tx.send(
            Event::default().event("Content").data(
                html! {
                    div.message__wrapper hx-swap-oob="outerHTML:#partial-system-message" {
                        div.message__bubble.message__bubble--system {
                            (system_message_markdown.to_html())
                        }
                        div.message__meta {
                            span.message_subdued { (crate::datetime::today_implied_readable_datetime(&message.created_at, &internationalization)) }
                            (render_feedback_form(message.id, None).unwrap())
                        }
                    }
                    (render_preview_oob(system_message_markdown.as_str()))
                }
                .into_string(),
            ),
        )
        .await
        .unwrap();

        // Note Safari will report an error when the client closes the EventSource. 🤷🏽‍♂️
        // Note for the client to process the event, we always need to send a data
        // payload.
        tx.send(
            Event::default()
                .event(END_OF_TRANSMISSION)
                .data(END_OF_TRANSMISSION),
        )
        .await
        .unwrap();
    });

    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response())
}
