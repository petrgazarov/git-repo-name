# git-repo-name

`git-repo-name` syncs repository name between your local git repo and remote. It extends `git` and works with GitHub and file remotes.

## Usage

`git-repo-name` provides three main commands:

```sh
git repo-name sync    # Sync local directory name with remote repository name
git repo-name fetch   # Fetch repository name from remote without making changes
git repo-name config  # Configure settings for git-repo-name
```

### Sync

Syncs the name between your local directory and remote repository based on the specified source of truth.

```sh
git repo-name sync [OPTIONS]
```

#### Options

- `-s, --source <SOURCE>`: Specify the source of truth for the name sync [default: remote]

  - Possible values: `remote`, `local`
  - When set to `remote`, the local directory will be renamed to match the remote repository name
  - [WIP] When set to `local`, the remote will be renamed to match the local directory name

- `-r, --remote <REMOTE>`: Override the default git remote [default: origin]

  - Use this to specify a different remote if your repository has multiple remotes

- `-n, --dry-run`: Print actions without executing them

**Examples:**

```bash
# Use remote name as source of truth (default behavior)
git repo-name sync

# Use local directory name as source of truth instead
git repo-name sync -s local

# Use a different remote than origin
git repo-name sync -r upstream

# Preview what would happen without making changes
git repo-name sync -n

# Combine multiple options
git repo-name sync -s local -r upstream -n
```

### Fetch

Retrieves the repository name from the remote without making any changes.

```sh
git repo-name fetch [OPTIONS]
```

#### Options

- `-r, --remote <REMOTE>`: Override the default git remote [default: origin]
  - Use this to specify a different remote if your repository has multiple remotes

**Examples:**

```bash
# Get repository name from default remote (origin)
git repo-name fetch

# Get repository name from a specific remote
git repo-name fetch -r upstream
```

### Config

View or set configuration options for git-repo-name.

```sh
git repo-name config <KEY> [VALUE]
```

#### Arguments

- `KEY`: The configuration key to get or set
- `VALUE`: (Optional) The value to set for the configuration key. If not provided, displays the current value.

#### Available Configuration Keys

- `github-token`: GitHub personal access token for accessing private repositories and modifying repositories

  - **When it's needed**:
    - Required when working with private repositories or when using `sync -s local`.
  - **Best practice**: Use [GitHub's Fine-grained personal access tokens](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-fine-grained-personal-access-token) with:
    - **Metadata permission (read)**: To fetch private repository names and URLs
    - **Administration permission (write)**: To rename repositories on GitHub

- `default-remote`: The default remote to use when none is specified (defaults to "origin")

**Examples:**

```bash
# View a configuration value
git repo-name config default-remote

# Set a configuration value
git repo-name config default-remote upstream

# Set GitHub token
git repo-name config github-token ghp_your_token_here
```

## Installation

### Homebrew (recommended)

```bash
brew tap petrgazarov/git-repo-name
brew install git-repo-name
```

### Direct binaries

Pre-compiled binaries are available on the [Releases page](https://github.com/petrgazarov/git-repo-name/releases).

### Build from source

Alternatively, you can clone this repository and build from source using Cargo:

```bash
cargo install --git https://github.com/petrgazarov/git-repo-name.git
```

## Supported remote repositories

git-repo-name currently supports GitHub and file remotes.

## Thanks

[git-open](https://github.com/paulirish/git-open) was the original inspiration for this project.

## Contributing & Development

Please open an issue or submit a PR. Especially welcome are feature contributions and bug reports/fixes.
