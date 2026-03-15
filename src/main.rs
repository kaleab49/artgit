mod storage;
mod repo;
mod ui;

use clap::{Parser, Subcommand};

use crate::repo::Repo;
use crate::storage::Error;

#[derive(Parser)]
#[command(name = "artgit")]
#[command(about = "Local version control for creative files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new artgit repository in the current directory
    Init,
    /// Commit current changes with a message
    Commit {
        /// Commit message
        #[arg(short = 'm', long = "message")]
        message: String,
    },
    /// Show status of files in the working directory
    Status,
    /// Show commit log
    Log,
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            let cwd = std::env::current_dir()?;
            let repo = Repo::init(&cwd)?;
            println!(
                "Initialized empty artgit repository in {}",
                repo.root().display()
            );
            Ok(())
        }
        Commands::Commit { message } => {
            let cwd = std::env::current_dir()?;
            let mut repo = Repo::open(&cwd)?;
            let commit = repo.commit(&message)?;
            println!(
                "Created commit {} - {}",
                &commit.id[..7.min(commit.id.len())],
                commit.message
            );
            Ok(())
        }
        Commands::Status => {
            let cwd = std::env::current_dir()?;
            let repo = Repo::open(&cwd)?;
            let status = repo.status()?;
            ui::print_status(&status);
            Ok(())
        }
        Commands::Log => {
            let cwd = std::env::current_dir()?;
            let repo = Repo::open(&cwd)?;
            let commits = repo.log();
            ui::print_log(commits);
            Ok(())
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
