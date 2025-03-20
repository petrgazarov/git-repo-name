# git-repo-name

`git-repo-name` is a CLI tool that syncs your local git directory name with the remote repository name. It simplifies the process of renaming repositories, supporting bidirectional syncing with a familiar push/pull command syntax.

You can use it to:

- rename a repo on GitHub, then run `git repo-name pull` to update the local git directory name
- rename a local git directory, then run `git repo-name push` to rename the GitHub repo

## Detailed Usage

`git-repo-name` provides four main commands:

```sh
git repo-name pull    # Fetch repo name from the remote and rename local git directory name to match it
git repo-name push    # Rename repo name on the remote with the local git directory name
git repo-name fetch   # Fetch repo name from the remote without making changes
git repo-name config  # Configure settings (GitHub token and default remote)
```

### pull

Renames the local git directory name with the remote repository name.

_Note: For private GitHub repos, this requires a GitHub PAT (see [Configuration Keys](#configuration-keys) for more details)._

**Examples:**

```bash
# Basic usage
git repo-name pull

# Specify a remote [default: origin]
git repo-name pull -r upstream

# Preview what would happen without making changes
git repo-name pull -n
```

### push

Updates the repository name on the remote with the local root directory name.

_Note: For GitHub repos, this requires a GitHub PAT (see [Configuration Keys](#configuration-keys) for more details)._

**Examples:**

```bash
# Basic usage
git repo-name push

# Specify a remote [default: origin]
git repo-name push -r upstream

# Preview what would happen without making changes
git repo-name push -n
```

### fetch

Retrieves the repository name from the remote without making any changes.

**Examples:**

```bash
# Basic usage
git repo-name fetch

# Specify a remote [default: origin]
git repo-name fetch -r upstream
```

### config

View or set configuration options.

#### Configuration Keys

- `default-remote`: The remote to use when none is specified (defaults to "origin")

  Examples:

  ```sh
  # View default remote
  git repo-name config default-remote

  # Set default remote
  git repo-name config default-remote upstream
  ```

- `github-token`: GitHub [personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens) for authenticating GitHub API requests.

  Note: See the table below to determine the type of GitHub token you need and the permissions required.

  | Scenario                                   | PAT type                | Permissions required                                                  |
  | ------------------------------------------ | ----------------------- | --------------------------------------------------------------------- |
  | Public repositories (pull/fetch only)      | N/A (no token required) | None                                                                  |
  | Private repositories owned by you          | Fine-grained            | Metadata (read) for `pull`/`fetch`; Administration (write) for `push` |
  | Organization repositories                  | Classic                 | repo scope                                                            |
  | Mixed personal & organization repositories | Classic                 | repo scope                                                            |

  Examples:

  ```sh
  # View GitHub token
  git repo-name config github-token

  # Set GitHub token
  git repo-name config github-token ghp_your_token_here
  ```

## Installation

### Install with Homebrew (recommended)

```bash
brew tap petrgazarov/git-repo-name
brew install git-repo-name
```

### Direct binaries

Pre-compiled binaries are available on the [Releases page](https://github.com/petrgazarov/git-repo-name/releases).

When downloading binaries directly, you'll need to manually set up shell integration:

1. Download both the binary and the shell script from the releases page
2. Make both files executable using:
   ```bash
   chmod +x /path/to/git-repo-name-bin /path/to/git-repo-name
   ```
3. Place the binary in your PATH as `git-repo-name-bin`
4. Place the shell script in your PATH as `git-repo-name`

### Build from source

Alternatively, you can clone this repository and build from source using Cargo:

```bash
cargo install --git https://github.com/petrgazarov/git-repo-name.git
```

Follow these steps to enable shell integration:

1. Download the shell script from the repository and make it executable:

   ```bash
   curl -o /usr/local/bin/git-repo-name https://raw.githubusercontent.com/petrgazarov/git-repo-name/main/shell/git-repo-name.sh
   chmod +x /usr/local/bin/git-repo-name
   ```

   Replace `/usr/local/bin` with your preferred installation directory (ensure it's in your PATH).

2. Rename the cargo-installed binary:
   ```bash
   mv $(which git-repo-name) $(dirname $(which git-repo-name))/git-repo-name-bin
   ```

## Supported remotes

`git-repo-name` Currently supports GitHub and file (bare) remotes. Contributions for GitLab, Bitbucket, and others are welcome!

## Acknowledgments

Inspired by [git-open](https://github.com/paulirish/git-open) — an awesome project you should check out.
