#![allow(dead_code)]

use assert_fs::TempDir;
use gag::BufferRedirect;
use ini::Ini;
use mockito;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Sets up a test config directory with a mock GitHub token.
/// Returns the path to the config directory.
pub fn setup_test_config(temp_dir: &Path) -> anyhow::Result<()> {
    let config_dir = temp_dir.join(".config/git-repo-name");
    std::fs::create_dir_all(&config_dir)?;
    std::env::set_var("XDG_CONFIG_HOME", temp_dir.join(".config"));
    let config_file = config_dir.join("config");

    let mut conf = Ini::new();
    conf.with_section(Some("github")).set("token", "mock-token");
    conf.with_section(None::<String>)
        .set("default_remote", "origin");
    conf.write_to_file(&config_file)?;

    Ok(())
}

/// Creates a bare repository in the given directory.
/// Returns the path to the repository.
pub fn create_bare_repo(temp: &TempDir, name: &str) -> anyhow::Result<PathBuf> {
    let repo_dir = temp.path().join(name);
    std::fs::create_dir_all(&repo_dir)?;
    git2::Repository::init_bare(&repo_dir)?;
    Ok(repo_dir)
}

/// Creates a main repository in the given directory.
/// Returns the path to the repository.
pub fn create_main_repo(temp: &TempDir, dir: &str) -> anyhow::Result<(PathBuf, git2::Repository)> {
    let repo_dir = temp.path().join(dir);
    std::fs::create_dir(&repo_dir)?;
    let repo = git2::Repository::init(&repo_dir)?;
    Ok((repo_dir, repo))
}

pub fn get_canonical_remote_url(repo_path: &Path) -> anyhow::Result<String> {
    let canonical_repo_path = repo_path.canonicalize()?;
    let canonical_remote_url = format!("file://{}", canonical_repo_path.display());
    Ok(canonical_remote_url)
}

/// Captures stdout while executing the given function and returns the captured output.
pub fn capture_stdout<F, R>(f: F) -> crate::Result<(String, R)>
where
    F: FnOnce() -> crate::Result<R>,
{
    let mut captured = String::new();
    let result = {
        let mut stdout = BufferRedirect::stdout().map_err(|e| crate::Error::Fs(e.to_string()))?;
        let result = f()?;
        stdout
            .read_to_string(&mut captured)
            .map_err(|e| crate::Error::Fs(e.to_string()))?;
        result
    };
    Ok((captured, result))
}

/// A RAII guard that restores the original working directory when dropped.
pub struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    pub fn new() -> Self {
        let original = std::env::current_dir().expect("Failed to get current working directory");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original)
            .expect("Failed to restore original working directory");
    }
}

/// Mock GitHub API response for a repository.
pub fn mock_github_repo(
    old_owner: &str,
    new_owner: &str,
    old_repo_name: &str,
    new_repo_name: &str,
) {
    let mut server = mockito::Server::new();
    std::env::set_var("GITHUB_API_BASE_URL", server.url());

    let response_body = serde_json::json!({
        "name": new_repo_name,
        "full_name": format!("{}/{}", new_owner, new_repo_name),
        // GitHub API always returns HTTPS URLs regardless of the request URL format
        "clone_url": format!("https://github.com/{}/{}.git", new_owner, new_repo_name)
    });

    let _mock = server
        .mock(
            "GET",
            format!("/repos/{}/{}", old_owner, old_repo_name).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body.to_string())
        .create();

    // Server will be kept alive until it goes out of scope at the end of the test
    std::mem::forget(server);
}

/// Mock GitHub API error response.
pub fn mock_github_error(owner: &str, repo: &str, status: usize) {
    let mut server = mockito::Server::new();
    std::env::set_var("GITHUB_API_BASE_URL", server.url());

    let _mock = server
        .mock("GET", format!("/repos/{}/{}", owner, repo).as_str())
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

    // Server will be kept alive until it goes out of scope at the end of the test
    std::mem::forget(server);
}
