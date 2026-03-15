use crate::repo::{BranchInfo, DiffReport, Repo, StatusReport};
use crate::storage::Commit;

pub fn print_status(report: &StatusReport) {
    if report.new.is_empty() && report.modified.is_empty() && report.unchanged.is_empty() {
        println!("No tracked files yet.");
        return;
    }

    if !report.new.is_empty() {
        println!("New files:");
        for path in &report.new {
            println!("  + {path}");
        }
    }

    if !report.modified.is_empty() {
        println!();
        println!("Modified files:");
        for path in &report.modified {
            println!("  ~ {path}");
        }
    }

    if !report.unchanged.is_empty() {
        println!();
        println!("Unchanged files:");
        for path in &report.unchanged {
            println!("    {path}");
        }
    }
}

pub fn print_log(commits: &[Commit]) {
    if commits.is_empty() {
        println!("No commits yet.");
        return;
    }

    for commit in commits.iter().rev() {
        let short_id = &commit.id[..7.min(commit.id.len())];
        println!(
            "{} {} - {}",
            commit.timestamp.to_rfc3339(),
            short_id,
            commit.message
        );
    }
}

pub fn print_branches(branches: &[BranchInfo], current: &str) {
    if branches.is_empty() {
        println!("No branches defined. Current branch: {current}");
        return;
    }
    println!("Branches:");
    for b in branches {
        let marker = if b.name == current { "*" } else { " " };
        match b.head_index {
            Some(idx) => println!("{marker} {} (head #{idx})", b.name),
            None => println!("{marker} {} (no commits)", b.name),
        }
    }
}

pub fn print_timeline(commits: &[Commit]) {
    if commits.is_empty() {
        println!("No commits yet.");
        return;
    }

    let mut first = true;
    for commit in commits.iter().rev() {
        if !first {
            println!("|");
        }
        first = false;
        let short_id = &commit.id[..7.min(commit.id.len())];
        println!(
            "* [{}] {} - {}",
            commit.timestamp.to_rfc3339(),
            short_id,
            commit.message
        );
    }
}

pub fn print_diff(report: &DiffReport) {
    if report.files.is_empty() {
        println!("No changes.");
        return;
    }

    for file in &report.files {
        println!("diff -- {}", file.path);
        if file.is_binary {
            println!("  (binary file changed)");
        } else if let Some(ref d) = file.diff {
            println!("{d}");
        }
        println!();
    }
}

#[cfg(feature = "tui")]
pub fn run_tui(_repo: &Repo) -> Result<(), Box<dyn std::error::Error>> {
    // Minimal placeholder: real ratatui UI can be added later.
    // For now, just print the log in a simple way so the command does something.
    // A full TUI would set up terminal raw mode and draw frames.
    println!("TUI mode is not fully implemented yet, showing simple timeline:");
    Ok(())
}

