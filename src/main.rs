use clap::{Parser, Subcommand};
use git_repo_name::{config::CONFIG, fetch_repo_name, sync, Error, Result, Source};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch repository name from remote
    Fetch {
        /// Override the default remote
        #[arg(short = 'r', long)]
        remote: Option<String>,
    },

    /// Sync local directory name with remote repository name
    Sync {
        /// Specify whether to use remote or local name as source of truth [default: remote]
        #[arg(short, long, value_enum, default_value_t = Source::Remote)]
        source: Source,

        /// Override the default git remote [default: origin]
        #[arg(short = 'r', long)]
        remote: Option<String>,

        /// Print actions without executing them
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Configure settings
    Config {
        /// Configuration key
        key: String,

        /// Configuration value (optional for getters)
        value: Option<String>,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fetch { remote } => {
            if let Some(remote_name) = remote {
                CONFIG.set_remote(remote_name);
            }
            fetch_repo_name()?;
            Ok(())
        }
        Commands::Sync {
            source,
            dry_run,
            remote,
        } => {
            if let Some(remote_name) = remote {
                CONFIG.set_remote(remote_name);
            }
            sync(source, dry_run)
        }
        Commands::Config { key, value } => match key.as_str() {
            "github-token" => match value {
                Some(token) => {
                    CONFIG.set_github_token(&token)?;
                    println!("GitHub token configured successfully");
                    Ok(())
                }
                None => {
                    let token = CONFIG.get_github_token()?;
                    println!("{}", token);
                    Ok(())
                }
            },
            "default-remote" => match value {
                Some(remote) => {
                    CONFIG.set_default_remote(&remote)?;
                    println!("Default remote set to {}", remote);
                    Ok(())
                }
                None => {
                    let remote = CONFIG.get_default_remote()?;
                    println!("{}", remote);
                    Ok(())
                }
            },
            _ => Err(Error::Config(format!(
                "Unknown config key: {}. Valid keys: github-token, default-remote",
                key
            ))),
        },
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
