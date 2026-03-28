use std::path::Path;

use anyhow::Context;

/// Extract text from a plain text file (UTF-8).
pub fn extract(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path).context("Failed to read text file")
}
