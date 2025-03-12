use crate::types::{Error, Result};
use regex::Regex;

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

pub fn format_new_remote_url(original_remote_url: &str, owner: &str, repo_name: &str) -> String {
    if original_remote_url.starts_with("git@") {
        // SSH shorthand (e.g. git@github.com:owner/repo.git)
        format!("git@github.com:{}/{}.git", owner, repo_name)
    } else if original_remote_url.starts_with("ssh://") {
        // Full SSH URL (e.g. ssh://git@github.com/owner/repo.git)
        format!("ssh://git@github.com/{}/{}.git", owner, repo_name)
    } else if original_remote_url.starts_with("git://") {
        // Git protocol (e.g. git://github.com/owner/repo.git)
        format!("git://github.com/{}/{}.git", owner, repo_name)
    } else {
        // Otherwise default to HTTPS.
        format!("https://github.com/{}/{}.git", owner, repo_name)
    }
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

    #[test]
    fn test_format_new_remote_url() {
        let cases = vec![
            // (original_remote_url, owner, repo_name, expected_remote_url)
            (
                "git@github.com:oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "git@github.com:newowner/newrepo.git",
            ),
            (
                "ssh://git@github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "ssh://git@github.com/newowner/newrepo.git",
            ),
            (
                "git://github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "git://github.com/newowner/newrepo.git",
            ),
            (
                "https://github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "https://github.com/newowner/newrepo.git",
            ),
            (
                "https://github.com/oldowner/oldrepo",
                "newowner",
                "newrepo",
                "https://github.com/newowner/newrepo.git",
            ),
            (
                "http://github.com/oldowner/oldrepo.git",
                "newowner",
                "newrepo",
                "https://github.com/newowner/newrepo.git",
            ),
        ];
        for (original, owner, repo_name, expected) in cases {
            assert_eq!(format_new_remote_url(original, owner, repo_name), expected);
        }
    }
}
