use {
    anyhow::{Context, Result},
    std::path::{Path, PathBuf},
};

/// Canonicalize a path.
pub fn canonicalize_path(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("Failed to canonicalize input directory: {}", path.display()))
}

/// Set up tracing subscriber to log with relative time.
pub fn setup_tracing_subscriber() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_timer(tracing_subscriber::fmt::time::Uptime::default())
        .init();
}
