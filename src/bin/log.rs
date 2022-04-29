extern crate git_rs;

use git_rs::stores::{fs as gitfs};
use git_rs::refs::RefSet;

pub fn main() -> std::io::Result<()> {
    let current_dir = std::env::current_dir()?;
    let storage_set = gitfs::from(current_dir.as_path())?;
    let ref_set = RefSet::from_path(current_dir.as_path())?;
    let args: Vec<String> = std::env::args().collect();

    let query = if args.len() < 2 {
        "HEAD"
    } else {
        &args[1]
    };

    let id = match ref_set.deref(query) {
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
        let lines: Vec<&str> = message.split('\n').collect();
        println!("\x1b[33m{} \x1b[0m{}", id, lines[0]);
    };

    Ok(())
}
