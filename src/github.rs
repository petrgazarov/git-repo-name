use crate::{config::CONFIG, Error, Result};
use git2::Repository;
use regex::Regex;
use reqwest::blocking::Client as ReqwestClient;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GitHubRepo {
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
}

pub fn get_repo_info(owner: &str, repo: &str) -> Result<GitHubRepo> {
    let client = create_client()?;
    let base_url = std::env::var("GITHUB_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.github.com".to_string());
    let url = format!("{}/repos/{}/{}", base_url, owner, repo);

    client
        .get(&url)
        .send()
        .map_err(|e| Error::GitHubApi(e.to_string()))?
        .error_for_status()
        .map_err(|e| Error::GitHubApi(e.to_string()))?
        .json()
        .map_err(|e| Error::GitHubApi(e.to_string()))
}

fn create_client() -> Result<ReqwestClient> {
    let token = CONFIG.get_github_token()?;
    let mut headers = HeaderMap::new();

    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("token {}", token))
            .map_err(|e| Error::GitHubApi(e.to_string()))?,
    );

    headers.insert(USER_AGENT, HeaderValue::from_static("git-repo-name"));

    ReqwestClient::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| Error::GitHubApi(e.to_string()))
}

pub fn is_github_url(url: &str) -> bool {
    let re = Regex::new(r"^(?:https://(?:www\.)?github\.com/|git@github\.com:|ssh://git@github\.com/|git://github\.com/)[^/]+/[^/\s]+(?:\.git)?$").unwrap();
    re.is_match(url)
}

pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    let re = Regex::new(r"^(?:https://(?:www\.)?github\.com/|git@github\.com:|ssh://git@github\.com/|git://github\.com/)([^/]+)/([^/\.]+?)(?:\.git)?$").unwrap();

    let caps = re
        .captures(url)
        .ok_or_else(|| Error::InvalidGitHubUrl(url.to_string()))?;

    let owner = caps
        .get(1)
        .ok_or_else(|| Error::InvalidGitHubUrl(url.to_string()))?
        .as_str()
        .to_string();

    let repo = caps
        .get(2)
        .ok_or_else(|| Error::InvalidGitHubUrl(url.to_string()))?
        .as_str()
        .to_string();

    Ok((owner, repo))
}

pub fn sync_github_repo(repo: &Repository, remote_url: &str, dry_run: bool) -> Result<()> {
    let (owner, repo_name) = parse_github_url(remote_url)?;
    let repo_info = get_repo_info(&owner, &repo_name)?;
    let new_name = repo_info.name;

    if dry_run {
        println!("Would rename repository to: {}", new_name);
        return Ok(());
    }

    if repo_info.clone_url != remote_url {
        let remote = CONFIG.get_remote()?;
        println!("Updating remote URL to: {}", repo_info.clone_url);
        crate::git::set_remote_url(repo, &remote, &repo_info.clone_url)?;
    }

    let repo_path = repo
        .workdir()
        .ok_or_else(|| Error::Fs("Cannot get repository working directory".into()))?;
    crate::fs::rename_directory(repo_path, &new_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url() {
        let test_cases = vec![
            // HTTPS URLs
            ("https://github.com/owner/repo.git", ("owner", "repo")),
            ("https://github.com/owner/repo", ("owner", "repo")),
            ("https://www.github.com/owner/repo.git", ("owner", "repo")),
            ("https://www.github.com/owner/repo", ("owner", "repo")),
            // SSH URLs
            ("git@github.com:owner/repo.git", ("owner", "repo")),
            ("git@github.com:owner/repo", ("owner", "repo")),
            ("ssh://git@github.com/owner/repo.git", ("owner", "repo")),
            ("ssh://git@github.com/owner/repo", ("owner", "repo")),
            // Git protocol URLs
            ("git://github.com/owner/repo.git", ("owner", "repo")),
            ("git://github.com/owner/repo", ("owner", "repo")),
        ];

        for (url, (expected_owner, expected_repo)) in test_cases {
            let (owner, repo) = parse_github_url(url).unwrap();
            assert_eq!(owner, expected_owner);
            assert_eq!(repo, expected_repo);
        }
    }

    #[test]
    fn test_is_github_url() {
        // Valid URLs
        assert!(is_github_url("https://github.com/owner/repo.git"));
        assert!(is_github_url("https://github.com/owner/repo"));
        assert!(is_github_url("https://www.github.com/owner/repo.git"));
        assert!(is_github_url("https://www.github.com/owner/repo"));
        assert!(is_github_url("git@github.com:owner/repo.git"));
        assert!(is_github_url("git@github.com:owner/repo"));
        assert!(is_github_url("ssh://git@github.com/owner/repo.git"));
        assert!(is_github_url("ssh://git@github.com/owner/repo"));
        assert!(is_github_url("git://github.com/owner/repo.git"));
        assert!(is_github_url("git://github.com/owner/repo"));

        // Invalid URLs
        assert!(!is_github_url("https://gitlab.com/owner/repo.git"));
        assert!(!is_github_url("git@gitlab.com:owner/repo.git"));
        assert!(!is_github_url("https://github.com"));
        assert!(!is_github_url("git@github.com:"));
    }
}
