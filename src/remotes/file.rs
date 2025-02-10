use crate::{config::CONFIG, fs, git, Error, Result};
use git2::Repository;
use path_clean::PathClean;
use std::path::Path;

pub fn sync_from_file_remote(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let local_directory_name = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .file_name()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_str()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_string();
    let canonical_path = resolve_canonical_path(Path::new(&remote_url))?;
    let resolved_repo_name = git::extract_repo_name_from_path(&canonical_path)?;

    let repo_path = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?;

    let resolved_remote_url = format_new_remote_url(remote_url, &canonical_path)?;
    let should_rename_directory = local_directory_name != resolved_repo_name;
    let should_change_remote = resolved_remote_url != remote_url;

    if !should_rename_directory && !should_change_remote {
        println!("Directory name and remote URL already up-to-date");
        return Ok(());
    }

    if should_change_remote {
        let remote = CONFIG.get_remote()?;
        if dry_run {
            println!(
                "Would change '{}' remote from '{}' to '{}'",
                remote, remote_url, resolved_remote_url
            );
        } else {
            println!(
                "Changing '{}' remote from '{}' to '{}'",
                remote, remote_url, resolved_remote_url
            );
            git::set_remote_url(repo, &remote, &resolved_remote_url)?;
        }
    }

    if should_rename_directory {
        fs::rename_directory(repo_path, &resolved_repo_name, dry_run)?;
    }

    Ok(())
}

/// Resolves a file path to its canonical form, following symlinks.
pub fn resolve_canonical_path(path: &Path) -> Result<String> {
    let path_str = path.to_string_lossy();
    let path_to_resolve = if path_str.starts_with("file://") {
        Path::new(&path_str[7..])
    } else {
        path
    };

    let canonical = path_to_resolve
        .canonicalize()
        .map_err(|e| Error::Fs(format!("Failed to resolve path: {}", e)))?;

    Ok(format!("file://{}", canonical.display()))
}

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
    use assert_fs::prelude::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::Path;

    #[test]
    fn test_resolve_canonical_path() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let real_dir = temp.child("real_dir");
        real_dir.create_dir_all()?;

        // Test regular path
        let resolved = resolve_canonical_path(real_dir.path())?;
        let expected = format!("file://{}", real_dir.path().canonicalize()?.display());
        assert_eq!(resolved, expected);

        // Test file:// URL
        let file_url = format!("file://{}", real_dir.path().display());
        let resolved_url = resolve_canonical_path(Path::new(&file_url))?;
        assert_eq!(resolved_url, expected);

        #[cfg(unix)]
        {
            let symlink_path = temp.child("link_dir");
            symlink(real_dir.path(), symlink_path.path())?;

            let resolved = resolve_canonical_path(symlink_path.path())?;
            assert_eq!(resolved, expected);
        }

        Ok(())
    }

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
