extern crate git_rs;

use git_rs::stores::{fs as gitfs};
use git_rs::stores::pack::{ Store as PackStore, GetObject };
use git_rs::stores::loose::{ Store as LooseStore };
use git_rs::objects::{ Type, Object };
use git_rs::packindex::Index;
use git_rs::errors::Result;
use git_rs::id::Id;

pub fn main() -> std::io::Result<()> {
    let storage_set = gitfs::from(std::env::current_dir()?.as_path())?;
    let args: Vec<String> = std::env::args().collect();

    let mut id = match Id::from_str(&args[1]) {
        Some(xs) => xs,
        None => return Ok(())
    };

    loop {
        let result = match storage_set.get_and_load(&id) {
            Err(_) => return Err(std::io::ErrorKind::InvalidData.into()),
            Ok(xs) => xs
        };

        let object = match result {
            Some(xs) => xs,
            None => return Err(std::io::ErrorKind::InvalidData.into())
        };

        match object {
            Object::Commit(xs) => {
                let message = std::str::from_utf8(&xs.message()).expect("not utf8");
                let lines: Vec<&str> = message.split("\n").collect();
                let parents = xs.parents();

                println!("{} {}", id, lines[0]);

                if parents.is_none() {
                    return Ok(());
                }

                let parent_vec = parents.unwrap();
                if parent_vec.len() < 1 {
                    return Ok(());
                }
                id = parent_vec[0].clone();
            },
            _ => {}
        }
    }

    Ok(())
}
