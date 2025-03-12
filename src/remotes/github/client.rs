use crate::{
    config::CONFIG,
    types::{Error, Result},
};
use reqwest::blocking::Client as ReqwestClient;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GitHubRepo {
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
}

pub fn create_client(token: Option<&str>) -> Result<ReqwestClient> {
    let mut headers = HeaderMap::new();

    // Add authorization header only if token is provided
    if let Some(token_str) = token {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("token {}", token_str))
                .map_err(|e| Error::GitHubApi(e.to_string()))?,
        );
    }

    headers.insert(USER_AGENT, HeaderValue::from_static("git-repo-name"));

    ReqwestClient::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| Error::GitHubApi(e.to_string()))
}

pub fn get_repo_info(owner: &str, repo: &str) -> Result<GitHubRepo> {
    let base_url = std::env::var("GITHUB_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.github.com".to_string());
    let url = format!("{}/repos/{}/{}", base_url, owner, repo);

    let token = CONFIG.get_github_token().ok();
    let client = create_client(token.as_deref())?;
    let response = client.get(&url).send();

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::NOT_FOUND {
                // GitHub returns 404 for private repos when unauthorized
                Err(Error::GitHubApi(
                  "Repository not found. If this is a private repository, please configure a GitHub token with 'git repo-name config github-token YOUR_TOKEN'".to_string(),
              ))
            } else {
                // Process successful response
                match resp.error_for_status() {
                    Ok(resp) => resp.json().map_err(|e| Error::GitHubApi(e.to_string())),
                    Err(e) => Err(Error::GitHubApi(e.to_string())),
                }
            }
        }
        Err(e) => Err(Error::GitHubApi(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_repo_info() -> anyhow::Result<()> {
        use crate::config::CONFIG;
        use crate::test_helpers;
        use assert_fs::TempDir;

        let temp = TempDir::new()?;
        test_helpers::setup_test_config(temp.path())?;

        let _guard = test_helpers::CurrentDirGuard::new();

        let owner = "test-owner";
        let repo = "test-repo";

        test_helpers::mock_github_repo(owner, owner, repo, repo);

        {
            CONFIG.set_github_token("")?;

            let result = get_repo_info(owner, repo);
            assert!(
                result.is_ok(),
                "Expected success for public repo with unauthenticated request"
            );
        }

        let private_repo = format!("{}-private", repo);
        test_helpers::mock_github_error(owner, &private_repo, 404);

        {
            let result = get_repo_info(owner, &private_repo);
            assert!(
                result.is_err(),
                "Expected error for private repo with unauthenticated request"
            );

            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("private repository"),
                "Error should mention private repository, got: {}",
                err
            );
        }

        Ok(())
    }
}
