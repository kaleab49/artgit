use crate::repo::StatusReport;
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

