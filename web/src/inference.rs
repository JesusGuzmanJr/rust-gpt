use {
    crate::{runner, thread::ThreadId, user::UserId},
    anyhow::Result,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InferenceJob {}

pub(crate) async fn launch_inference(
    user_id: UserId,
    thread_id: ThreadId,
    job: InferenceJob,
) -> Result<()> {
    tracing::info!(%user_id, %thread_id, "mocking launching inference");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    tracing::info!(%user_id, %thread_id, "mocking inference completed");
    runner::gpu_ready();
    Ok(())
}
