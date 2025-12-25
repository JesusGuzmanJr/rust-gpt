use {
    crate::{
        internationalization::Internationalization,
        message::{PartialSystemMessage, SystemMessage},
        pages::chat::{message_handlers::render_preview_oob, views::render_feedback_form},
    },
    anyhow::{Error, Result},
    axum::response::{
        Sse,
        sse::{Event, KeepAliveStream},
    },
    derive_more::Display,
    maud::html,
    tokio::sync::mpsc::Sender,
    tokio_stream::wrappers::ReceiverStream,
};

#[derive(Debug, Clone)]
pub(crate) struct SseSender(Sender<Result<Event, Error>>);

type SseReceiver = Sse<KeepAliveStream<ReceiverStream<Result<Event, Error>>>>;

pub(crate) const SYSTEM_STATE_SSE: &str = "SystemState";
pub(crate) const CONTENT_SSE: &str = "Content";

/// The Unicode End of Transmission (EOT) character U+0004.
pub(super) const END_OF_TRANSMISSION: &str = "\u{4}";

#[derive(Debug, Clone, Copy, Display)]
pub(crate) enum SystemStateLabel {
    #[display("Initializing...")]
    Initializing,

    #[display("Generating response...")]
    GeneratingResponse,
}

pub(crate) fn new_sse_channel() -> (SseSender, SseReceiver) {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event>>(10);
    (
        SseSender(tx),
        Sse::new(ReceiverStream::new(rx)).keep_alive(axum::response::sse::KeepAlive::default()),
    )
}
impl SseSender {
    pub(crate) async fn send_system_state(&self, state: SystemStateLabel) -> Result<()> {
        self.0
            .send(Ok(Event::default()
                .event(SYSTEM_STATE_SSE)
                .data(state.to_string())))
            .await
            .map_err(|_| anyhow::anyhow!("failed to send system state '{state}'; channel closed"))
    }

    pub(crate) async fn send_partial_system_message(
        &self,
        message: &PartialSystemMessage,
    ) -> Result<()> {
        self.0
            .send(Ok(Event::default().event(CONTENT_SSE).data(
                html! {
                    (message.content.to_html())
                    (render_preview_oob(message.content.as_str()))
                }
                .into_string(),
            )))
            .await
            .map_err(|_| anyhow::anyhow!("failed to send system message markdown; channel closed"))
    }

    pub(crate) async fn send_final_system_message(
        self,
        message: &SystemMessage,
        internationalization: &Internationalization,
    ) -> Result<()> {
        // send final message
        self.0.send(
            Ok(Event::default().event(CONTENT_SSE).data(
                html! {
                    div.message__wrapper hx-swap-oob="outerHTML:#partial-system-message" {
                        div.message__bubble.message__bubble--system {
                            (message.content.to_html())
                        }
                        div.message__meta {
                            span.message_subdued { (crate::datetime::today_implied_readable_datetime(&message.created_at, &internationalization)) }
                            (render_feedback_form(message.id, None).unwrap())
                        }
                    }
                    (render_preview_oob(message.content.as_str()))
                }
                .into_string(),
            )),
        )
        .await
    .map_err(|_| anyhow::anyhow!("failed to send final system message; channel closed"))?;

        // Safari will report an error when the client closes the EventSource. 🤷🏽‍♂️
        // Nothing we can do. This function is used by htmx API to close the
        // EventSource. Also, for the client to process the event, we always
        // need to send a data payload.
        self.0
            .send(Ok(Event::default()
                .event(END_OF_TRANSMISSION)
                .data(END_OF_TRANSMISSION)))
            .await
            .map_err(|_| anyhow::anyhow!("failed to send final system message; channel closed"))
    }
}
