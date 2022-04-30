extern crate git_rs;

use git_rs::stores::fs as gitfs;
use git_rs::refs::RefSet;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    cwd: Option<PathBuf>,
    branch: Option<String>,
}

pub fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let current_dir = args.cwd.or_else(|| std::env::current_dir().ok()).unwrap();
    let storage_set = gitfs::from(current_dir.as_path())?;
    let ref_set = RefSet::from_path(current_dir.as_path())?;

    let query = args.branch.unwrap_or_else(|| "HEAD".to_string());

    let id = match ref_set.deref(&query) {
        Some(result) => result.clone(),
        None => {
            match query.parse() {
                Ok(xs) => xs,
                Err(_) => return Ok(())
            }
        }
    };

    for (id, commit) in storage_set.commits(&id, None) {
        let message = std::str::from_utf8(commit.message()).expect("not utf8");

        let idx = message.find('\n').unwrap_or(message.len());

        println!("\x1b[33m{} \x1b[0m{}", id, &message[0..idx]);
    };

    Ok(())
}
