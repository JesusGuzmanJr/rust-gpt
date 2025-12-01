use {
    crate::{inference::InferenceRequest, round_robin::RoundRobin, thread::ThreadId, user::UserId},
    anyhow::Result,
    std::sync::{LazyLock, OnceLock, RwLock},
    tokio::sync::mpsc::UnboundedSender,
    tracing::instrument,
};

static HANDLE: OnceLock<UnboundedSender<Message>> = OnceLock::new();

fn handle() -> &'static UnboundedSender<Message> {
    HANDLE.get().expect("runner not initialized")
}

static CURRENT_THREAD_ID: LazyLock<RwLock<Option<ThreadId>>> = LazyLock::new(|| RwLock::new(None));

pub(crate) fn current_thread_id() -> Option<ThreadId> {
    *CURRENT_THREAD_ID.read().expect("poisoned RwLock")
}

fn set_current_thread_id(thread_id: ThreadId) {
    let _ = CURRENT_THREAD_ID
        .write()
        .expect("poisoned RwLock")
        .insert(thread_id);
}

enum Message {
    QueueRequest {
        user_id: UserId,
        thread_id: ThreadId,
        request: InferenceRequest,
    },
    GpuReady,
}

#[instrument]
pub(crate) fn queue_task(user_id: UserId, thread_id: ThreadId, request: InferenceRequest) {
    tracing::debug!(%user_id, %thread_id, "queueing inference request");
    handle()
        .send(Message::QueueRequest {
            user_id,
            thread_id,
            request,
        })
        .expect("failed to queue request; scheduler dropped");
}

pub(crate) fn gpu_ready() {
    handle()
        .send(Message::GpuReady)
        .expect("failed to send GPU ready message; scheduler dropped");
}

struct State {
    scheduler: RoundRobin<UserId, ThreadId, InferenceRequest>,
}

/// Start the scheduler in a new Tokio task.
pub(crate) fn init() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    HANDLE.set(sender).expect("scheduler already initialized");

    let mut state = State {
        scheduler: RoundRobin::new(),
    };

    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Err(error) = process_message(&mut state, message).await {
                tracing::error!(?error, "failed to process message");
            }
        }
        tracing::debug!("scheduler shutting down");
    });
}

async fn process_message(state: &mut State, message: Message) -> Result<()> {
    match message {
        Message::QueueRequest {
            user_id,
            thread_id,
            request,
        } => {
            state.scheduler.push(user_id, thread_id, request);
        }
        Message::GpuReady => {
            if let Some((user_id, thread_id, request)) = state.scheduler.pop() {
                launch_inference(user_id, thread_id, request).await?;
            } else {
                tracing::debug!("GPU ready, but no scheduled inference jobs");
            }
        }
    }
    Ok(())
}

pub(crate) async fn launch_inference(
    user_id: UserId,
    thread_id: ThreadId,
    request: InferenceRequest,
) -> Result<()> {
    tracing::info!(%user_id, %thread_id, "mocking launching inference");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    tracing::info!(%user_id, %thread_id, "mocking inference completed");
    gpu_ready();
    Ok(())
}
