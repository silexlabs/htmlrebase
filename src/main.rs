use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use htmlrebase::{rebase_dir, Prefix};

/// Add a base URL to a built static site: rewrite root-relative URLs in HTML and CSS, in place.
#[derive(Parser, Debug)]
#[command(name = "htmlrebase", version, about, long_about = None)]
struct Cli {
    /// Folder of the built site. Files are rewritten in place.
    dir: PathBuf,

    /// Path to add in front of every root-relative URL: /repo/, /repo or repo.
    #[arg(long)]
    prefix: Prefix,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stats = match rebase_dir(&cli.dir, &cli.prefix) {
        Ok(stats) => stats,
        Err(e) => {
            eprintln!("htmlrebase: error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if cli.prefix.is_root() {
        println!("htmlrebase: prefix is /, nothing to do");
        return ExitCode::SUCCESS;
    }
    for (path, reason) in &stats.skipped {
        eprintln!("htmlrebase: warning: skipped {}: {reason}", path.display());
    }
    println!(
        "htmlrebase: {} of {} files changed, {} skipped, prefix {}",
        stats.changed.len(),
        stats.scanned,
        stats.skipped.len(),
        cli.prefix
    );
    if stats.skipped.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
