use clap::{Parser, Subcommand};
use git_repo_name::{
    config::CONFIG,
    fetch_repo_name, pull, push,
    types::{Error, Result},
};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Fetch {
        #[arg(short = 'r', long)]
        remote: Option<String>,
    },

    Pull {
        #[arg(short = 'r', long)]
        remote: Option<String>,

        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    Push {
        #[arg(short = 'r', long)]
        remote: Option<String>,

        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    Config {
        key: String,

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
        Commands::Pull { remote, dry_run } => {
            if let Some(remote_name) = remote {
                CONFIG.set_remote(remote_name);
            }
            pull(dry_run)
        }
        Commands::Push { remote, dry_run } => {
            if let Some(remote_name) = remote {
                CONFIG.set_remote(remote_name);
            }
            push(dry_run)
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
