use {
    anyhow::{Context, Result},
    std::path::{Path, PathBuf},
};

/// Canonicalize a path.
pub fn canonicalize_path(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("Failed to canonicalize input directory: {}", path.display()))
}
