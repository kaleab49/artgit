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
    /// List or create branches
    Branch {
        /// Optional branch name to create; if omitted, list branches
        name: Option<String>,
    },
    /// Switch current branch
    Switch {
        /// Branch name to switch to
        name: String,
    },
    /// Show diff between working tree and HEAD
    Diff,
    /// Launch TUI (if built with `tui` feature)
    #[cfg(feature = "tui")]
    Tui,
    /// Create a bundle file containing commits and objects
    BundleCreate {
        /// Output bundle file path
        file: String,
    },
    /// Apply a bundle file to the current repo
    BundleApply {
        /// Input bundle file path
        file: String,
    },
    /// Restore a file from the latest commit
    Restore {
        /// Path to restore (relative to repo root)
        path: String,
    },
    /// Checkout a commit (optionally a single path)
    Checkout {
        /// Commit id or prefix
        commit: String,
        /// Optional file path to restore from that commit
        #[arg(long)]
        path: Option<String>,
    },
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
        Commands::Branch { name } => {
            let cwd = std::env::current_dir()?;
            let mut repo = Repo::open(&cwd)?;
            match name {
                Some(n) => {
                    repo.create_branch(&n)?;
                    println!("Created branch {n}");
                }
                None => {
                    let branches = repo.list_branches();
                    ui::print_branches(&branches, repo.current_branch());
                }
            }
            Ok(())
        }
        Commands::Switch { name } => {
            let cwd = std::env::current_dir()?;
            let mut repo = Repo::open(&cwd)?;
            repo.switch_branch(&name)?;
            println!("Switched to branch {name}");
            Ok(())
        }
        Commands::Diff => {
            let cwd = std::env::current_dir()?;
            let repo = Repo::open(&cwd)?;
            let report = repo.diff_working_vs_head()?;
            ui::print_diff(&report);
            Ok(())
        }
        #[cfg(feature = "tui")]
        Commands::Tui => {
            let cwd = std::env::current_dir()?;
            let repo = Repo::open(&cwd)?;
            ui::run_tui(&repo)?;
            Ok(())
        }
        Commands::BundleCreate { file } => {
            let cwd = std::env::current_dir()?;
            let repo = Repo::open(&cwd)?;
            let bundle = repo.create_bundle()?;
            let data = serde_json::to_vec_pretty(&bundle)?;
            std::fs::write(&file, data)?;
            println!("Wrote bundle to {file}");
            Ok(())
        }
        Commands::BundleApply { file } => {
            let cwd = std::env::current_dir()?;
            let mut repo = Repo::open(&cwd)?;
            let data = std::fs::read(&file)?;
            let bundle: repo::Bundle = serde_json::from_slice(&data)?;
            repo.apply_bundle(bundle)?;
            println!("Applied bundle from {file}");
            Ok(())
        }
        Commands::Restore { path } => {
            let cwd = std::env::current_dir()?;
            let repo = Repo::open(&cwd)?;
            repo.restore_file(&path)?;
            println!("Restored {path} from latest commit");
            Ok(())
        }
        Commands::Checkout { commit, path } => {
            let cwd = std::env::current_dir()?;
            let mut repo = Repo::open(&cwd)?;
            repo.checkout_commit(&commit, path.as_deref())?;
            println!("Checked out {commit}");
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
