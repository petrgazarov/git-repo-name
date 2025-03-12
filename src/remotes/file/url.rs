use crate::types::{Error, Result};
use path_clean::PathClean;
use std::path::Path;

/// Formats a new path from a canonical path, keeping the format of the original remote URL.
pub fn format_new_remote_url(original_remote_url: &str, canonical_path: &str) -> Result<String> {
    // If the original URL is relative and it is equivalent to the given canonical_path (without canonicalization),
    // then just return the original URL.
    let original_path = Path::new(original_remote_url);
    if original_path.is_relative() {
        let joined = std::env::current_dir()?.join(original_path);
        let normalized = joined.clean();
        let normalized_str = normalized
            .to_str()
            .ok_or_else(|| Error::Fs("Failed to convert path to string".into()))?;
        let expanded_full = format!("file://{}", normalized_str);
        if expanded_full == canonical_path {
            return Ok(original_remote_url.to_string());
        }
    }

    // Otherwise, format based on whether the original URL has a file:// prefix.
    if original_remote_url.trim_start().starts_with("file://") {
        Ok(canonical_path.to_string())
    } else {
        Ok(canonical_path.trim_start_matches("file://").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_new_remote_url() -> anyhow::Result<()> {
        // Calculate canonical path for relative path test
        let current_dir = std::env::current_dir()?;
        let norm = current_dir.join("repo.git").clean();
        let norm_str = norm
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Conversion error"))?;
        let canonical_expected = format!("file://{}", norm_str);

        let test_cases = vec![
            // (original_remote_url, canonical_path, expected_result)
            (
                "file:///old/path/repo.git",
                "file:///new/path/repo.git",
                "file:///new/path/repo.git",
            ),
            (
                "/old/path/repo.git",
                "file:///new/path/repo.git",
                "/new/path/repo.git",
            ),
            // When canonical path matches the expanded original path
            ("repo.git", &canonical_expected, "repo.git"),
            // When canonical path is different from the expanded original path
            (
                "repo.git",
                "file:///different/path/repo.git",
                "/different/path/repo.git",
            ),
        ];

        for (original, canonical, expected) in test_cases {
            let result = format_new_remote_url(original, canonical)?;
            assert_eq!(result, expected);
        }

        Ok(())
    }
}
