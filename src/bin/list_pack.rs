extern crate git_rs;

use std::fs::File;
use std::io::BufReader;

use git_rs::stores::{fs as gitfs};
use git_rs::pack::iter::PackfileIterator;

pub fn main() -> std::io::Result<()> {
    let current_dir = std::env::current_dir()?;
    let storage_set = gitfs::from(current_dir.as_path())?;
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("must provide a filename");
        return Ok(())
    }

    let f = File::open(&args[1])?;

    for entry in PackfileIterator::new(BufReader::new(f), Some(&storage_set)).expect("failed to parse as packfile") {
        println!("{} {} ({:x})", entry.offset(), entry.id(), entry.crc32());
    }
    Ok(())
}

