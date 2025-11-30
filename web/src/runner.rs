use {
    crate::{
        inference::{InferenceJob, launch_inference},
        scheduler::RoundRobin,
        thread::ThreadId,
        user::UserId,
    },
    anyhow::Result,
    std::sync::OnceLock,
    tokio::sync::mpsc::UnboundedSender,
};

static HANDLE: OnceLock<UnboundedSender<Message>> = OnceLock::new();

fn handle() -> &'static UnboundedSender<Message> {
    HANDLE.get().expect("runner not initialized")
}

enum Message {
    QueueTask {
        user_id: UserId,
        thread_id: ThreadId,
        job: InferenceJob,
    },
    GpuReady,
}

pub(crate) fn queue_task(user_id: UserId, thread_id: ThreadId, job: InferenceJob) {
    handle()
        .send(Message::QueueTask {
            user_id,
            thread_id,
            job,
        })
        .expect("failed to queue task; runner dropped");
}

pub(crate) fn gpu_ready() {
    handle()
        .send(Message::GpuReady)
        .expect("failed to send GPU ready message; runner dropped");
}

struct State {
    scheduler: RoundRobin,
}

/// Start the runner in a new Tokio task.
pub(crate) fn init_runner() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    HANDLE.set(sender).expect("runner already initialized");

    let mut state = State {
        scheduler: RoundRobin::new(),
    };

    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Err(error) = process_message(&mut state, message).await {
                tracing::error!(?error, "failed to process message");
            }
        }
        tracing::debug!("runner shutting down");
    });
}

async fn process_message(state: &mut State, message: Message) -> Result<()> {
    match message {
        Message::QueueTask {
            user_id,
            thread_id,
            job,
        } => {
            state.scheduler.push(user_id, thread_id, job);
        }
        Message::GpuReady => {
            if let Some((user_id, thread_id, job)) = state.scheduler.pop() {
                launch_inference(user_id, thread_id, job).await?;
            } else {
                tracing::debug!("GPU ready, but no scheduled inference jobs");
            }
        }
    }
    Ok(())
}
