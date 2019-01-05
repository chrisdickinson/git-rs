extern crate git_rs;

use git_rs::stores::{fs as gitfs};
use git_rs::stores::pack::{ Store as PackStore, GetObject };
use git_rs::stores::loose::{ Store as LooseStore };
use git_rs::objects::{ Type, Object };
use git_rs::packindex::Index;
use git_rs::errors::Result;
use git_rs::refs::RefSet;
use git_rs::id::Id;

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

    let mut id = match ref_set.deref(&query) {
        Some(result) => result.clone(),
        None => {
            match Id::from_str(&query) {
                Some(xs) => xs,
                None => return Ok(())
            }
        }
    };

    for (id, commit) in storage_set.commits(&id, None) {
        let message = std::str::from_utf8(&commit.message()).expect("not utf8");
        let lines: Vec<&str> = message.split("\n").collect();
        println!("\x1b[33m{} \x1b[0m{}", id, lines[0]);
    };

    Ok(())
}
