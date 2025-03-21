# git-repo-name

`git-repo-name` is a CLI tool that syncs your local git directory name with the remote repository name.

It works bi-directionally and supports these two main use cases:

- When you rename a repo on GitHub, run `git repo-name pull` to update the local git directory name.
- When you rename a local git directory, run `git repo-name push` to rename the repo on GitHub.

In both cases, it makes an API call to GitHub, compares the repo name to the local directory name, and automatically renames the appropriate side.

## Detailed Usage

`git-repo-name` provides four main commands:

```sh
git repo-name pull    # Fetches repo name from the remote and renames local git directory name to match it
git repo-name push    # Renames repo name on the remote with the local git directory name
git repo-name fetch   # Fetches repo name from the remote without making changes
git repo-name config  # Configures settings (GitHub token and default remote)
```

### pull

Fetches repo name from the remote and renames local git directory name to match it.

_Note: For private GitHub repos, this requires a GitHub PAT (see [Configuration Keys](#configuration-keys) for more details)._

Examples

```bash
# Basic usage
git repo-name pull

# Specify a remote [default: origin]
git repo-name pull -r upstream

# Preview what would happen without making changes
git repo-name pull -n
```

### push

Renames repo name on the remote with the local git directory name.

_Note: For GitHub repos, this requires a GitHub PAT (see [Configuration Keys](#configuration-keys) for more details)._

Examples

```bash
# Basic usage
git repo-name push

# Specify a remote [default: origin]
git repo-name push -r upstream

# Preview what would happen without making changes
git repo-name push -n
```

### fetch

Fetches repo name from the remote without making changes.

Examples

```bash
# Basic usage
git repo-name fetch

# Specify a remote [default: origin]
git repo-name fetch -r upstream
```

### config

Configures settings.

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
  | Public repositories (`pull`/`fetch` only)  | N/A (no token required) | None                                                                  |
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

1. Install the `git-repo-name` formula:

   ```bash
   brew tap petrgazarov/git-repo-name
   brew install git-repo-name
   ```

2. Add the following line to your shell startup file (e.g., `~/.bashrc` or `~/.zshrc`):

   ```sh
   source "$(brew --prefix)/share/git-repo-name/git-repo-name.sh"
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
5. Add the following line to your shell startup file (e.g., `~/.bashrc` or `~/.zshrc`):

   ```sh
   source "$(which git-repo-name)"
   ```

### Build from source

Alternatively, you can clone this repository and build from source using Cargo:

1. Install and build the binary:

   ```bash
   cargo install --git https://github.com/petrgazarov/git-repo-name.git
   ```

2. Download the shell script from the repository and make it executable:

   ```bash
   curl -o /usr/local/bin/git-repo-name https://raw.githubusercontent.com/petrgazarov/git-repo-name/main/shell/git-repo-name.sh
   chmod +x /usr/local/bin/git-repo-name
   ```

   Replace `/usr/local/bin` with your preferred installation directory (ensure it's in your PATH).

3. Rename the cargo-installed binary:

   ```bash
   mv $(which git-repo-name) $(dirname $(which git-repo-name))/git-repo-name-bin
   ```

4. Add the following line to your shell startup file (e.g., `~/.bashrc` or `~/.zshrc`):

   ```sh
   source "$(which git-repo-name)"
   ```

## Supported remotes

`git-repo-name` currently supports GitHub and file (bare) remotes.

## Acknowledgments

Inspired by [git-open](https://github.com/paulirish/git-open) — an awesome project you should check out.
