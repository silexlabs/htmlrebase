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
    if cli.prefix.is_root() {
        println!("htmlrebase: prefix is /, nothing to do");
        return ExitCode::SUCCESS;
    }
    match rebase_dir(&cli.dir, &cli.prefix) {
        Ok(stats) => {
            println!(
                "htmlrebase: {} of {} files changed, prefix {}",
                stats.changed.len(),
                stats.scanned,
                cli.prefix
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("htmlrebase: error: {e}");
            ExitCode::FAILURE
        }
    }
}
