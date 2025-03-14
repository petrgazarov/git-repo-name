use crate::{
    config::CONFIG,
    types::{Error, Result},
};
use git2::Repository;
use std::path::Path;

pub fn get_current_repo() -> Result<Repository> {
    Repository::discover(".").map_err(|_| Error::NotAGitRepo)
}

pub fn get_remote_url(repo: &Repository) -> Result<String> {
    let remote_name = CONFIG.get_remote()?;

    let remote = repo
        .find_remote(&remote_name)
        .map_err(|_| Error::NoRemote(remote_name.clone()))?;

    let url = remote
        .url()
        .ok_or_else(|| Error::NoRemote(remote_name.clone()))?
        .to_string();

    Ok(url)
}

pub fn set_remote_url(
    repo: &Repository,
    current_url: &str,
    new_url: &str,
    dry_run: bool,
) -> Result<()> {
    let remote_name = CONFIG.get_remote()?;

    if dry_run {
        println!(
            "Would change '{}' remote from '{}' to '{}'",
            remote_name, current_url, new_url
        );
    } else {
        println!(
            "Changing '{}' remote from '{}' to '{}'",
            remote_name, current_url, new_url
        );

        repo.remote_set_url(&remote_name, new_url)
            .map_err(|e| Error::Other(e.into()))?;
    }

    Ok(())
}

pub fn extract_repo_name_from_path(url: &str) -> Result<String> {
    let url = url.strip_suffix(".git").unwrap_or(url);

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

pub fn get_local_directory_name(repo: &Repository) -> Result<String> {
    let local_directory_name = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .file_name()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_str()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?
        .to_string();

    Ok(local_directory_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;
    use assert_fs::TempDir;

    #[test]
    fn test_extract_repo_name_from_path() {
        let test_cases = vec![
            ("/path/to/repo.git", "repo"),
            ("/path/to/repo", "repo"),
            ("repo.git", "repo"),
            ("repo", "repo"),
            ("file:///path/to/repo.git", "repo"),
            ("file:///path/to/repo", "repo"),
        ];

        for (url, expected) in test_cases {
            assert_eq!(extract_repo_name_from_path(url).unwrap(), expected);
        }
    }

    #[test]
    fn test_get_local_directory_name() -> anyhow::Result<()> {
        let temp = TempDir::new()?;

        let repo_name = "test-repo";
        let (_repo_path, repo) = test_helpers::create_main_repo(&temp, repo_name)?;
        let dir_name = get_local_directory_name(&repo).map_err(|e| anyhow::anyhow!("{}", e))?;

        assert_eq!(dir_name, repo_name);

        Ok(())
    }
}
