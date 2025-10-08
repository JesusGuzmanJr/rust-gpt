use {
    anyhow::{Context, Result},
    std::{
        io::IsTerminal,
        path::{Path, PathBuf},
    },
};

/// Set up tracing subscriber to log with relative time.
pub fn setup_tracing_subscriber() {
    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::Uptime::default())
        .with_ansi(std::io::stdout().is_terminal())
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
}
