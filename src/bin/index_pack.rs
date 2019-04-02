extern crate git_rs;

use memmap::MmapOptions;
use std::fs::File;
use std::io::{self, Cursor};

use git_rs::pack::index::write;
use git_rs::stores::fs as gitfs;

pub fn main() -> std::io::Result<()> {
    let current_dir = std::env::current_dir()?;
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("must provide a filename");
        return Ok(());
    }

    let f = File::open(&args[1])?;
    let mmap = unsafe { MmapOptions::new().map(&f)? };

    let cursor = Cursor::new(&mmap[..]);

    let storage_set = gitfs::from(current_dir.as_path()).expect("failed to open storage");
    write(cursor, &mut io::stdout(), Some(&storage_set));

    Ok(())
}
