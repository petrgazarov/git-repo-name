use crate::{Error, Result};
use git2::Repository;
use std::path::Path;

pub fn get_current_repo() -> Result<Repository> {
    Repository::discover(".").map_err(|_| Error::NotAGitRepo)
}

pub fn get_remote_url(repo: &Repository, remote_name: &str) -> Result<String> {
    let remote = repo
        .find_remote(remote_name)
        .map_err(|_| Error::NoRemote(remote_name.to_string()))?;

    let url = remote
        .url()
        .ok_or_else(|| Error::NoRemote(remote_name.to_string()))?
        .to_string();

    Ok(url)
}

pub fn set_remote_url(repo: &Repository, remote_name: &str, url: &str) -> Result<()> {
    repo.remote_set_url(remote_name, url)
        .map_err(|e| Error::Other(e.into()))
}

pub fn extract_repo_name_from_path(url: &str) -> Result<String> {
    // Remove .git suffix if present
    let url = url.strip_suffix(".git").unwrap_or(url);

    // Get the last component of the path
    let name = Path::new(url)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            Error::Other(anyhow::anyhow!(
                "Could not extract repository name from URL"
            ))
        })?;

    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_from_path() {
        let test_cases = vec![
            ("/path/to/repo.git", "repo"),
            ("/path/to/repo", "repo"),
            ("repo.git", "repo"),
            ("repo", "repo"),
            // Test with file:// URLs
            ("file:///path/to/repo.git", "repo"),
            ("file:///path/to/repo", "repo"),
        ];

        for (url, expected) in test_cases {
            assert_eq!(extract_repo_name_from_path(url).unwrap(), expected);
        }
    }
}
