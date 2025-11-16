use {
    super::{
        types::{FeedbackForm, SendForm, StreamQuery, UpdateMessageForm, END_OF_TRANSMISSION},
        views::{render_feedback_form, render_message},
    },
    crate::{
        error::AppResult,
        internationalization::Internationalization,
        message::{Message, Payload, SystemMessageContent},
    },
    axum::{
        extract::Query,
        response::{
            IntoResponse,
            sse::{Event, KeepAlive, Sse},
        },
        Form,
    },
    axum_valid::Garde,
    maud::html,
    std::convert::Infallible,
    tokio_stream::StreamExt,
    tracing::*,
};

#[instrument]
pub(super) async fn send_message(
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
    let response = render_message(&message, &internationalization)?.into_response();
    message.save().await?;
    Ok(response)
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
    let stream =
        tokio_stream::wrappers::ReceiverStream::new(rx).map(Result::<_, Infallible>::Ok);

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut content = String::from("The ");

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
                p {
                    (content)
                    // span.spinner-beachball {}
                }
            };
            tx.send(Event::default().event("Content").data(html.into_string()))
                .await
                .unwrap();

            content += " word";
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        message.payload = Payload::System {
            content: SystemMessageContent::new(content.clone()),
            feedback: None,
        };
        message.clone().save().await.unwrap();

        tx.send(
            Event::default().event("Content").data(
                html! {
                    div.message__wrapper hx-swap-oob="outerHTML:#partial-system-message" {
                        div.message__bubble.message__bubble--system {
                            (content)
                        }
                        div.message__meta {
                            span.message_subdued { (crate::datetime::today_implied_readable_datetime(&message.created_at, &internationalization)) }
                            (render_feedback_form(message.id, None).unwrap())
                        }
                    }
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

    // TODO: it should be 1 SSE connetion per user client!

    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response())
}
