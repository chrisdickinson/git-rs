extern crate git_rs;

use git_rs::stores::pack::{ Store as PackStore, GetObject };
use git_rs::stores::loose::{ Store as LooseStore };
use git_rs::objects::{ Type, Object };
use git_rs::packindex::Index;
use git_rs::errors::Result;
use git_rs::id::Id;

pub fn main() -> std::io::Result<()> {
    let path = std::env::current_dir()?;
    let path_str = path.to_str();
    if path_str.is_none() {
        return Ok(())
    }

    let path_buf: std::path::PathBuf = [path_str.unwrap(), ".git", "objects", "pack"].iter().collect();

    let mut stores: Vec<Box<GetObject>> = Vec::new();

    let root: std::path::PathBuf = [path_str.unwrap(), ".git", "objects"].iter().collect();
    let loose_store = LooseStore::new(move |id| {
        let as_str = id.to_string();
        let mut pb = root.clone();
        pb.push(as_str[0..2].to_string());
        pb.push(as_str[2..40].to_string());
        match std::fs::File::open(pb.as_path()) {
            Ok(f) => Ok(Some(Box::new(f))),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => return Ok(None),
                    _ => return Err(e)?
                }
            }
        }
    });
    stores.push(Box::new(move |id| {
        loose_store.get(id)
    }));

    for entry in std::fs::read_dir(path_buf.as_path())? {
        let entry = entry?;
        let os_filename = entry.file_name();
        let filename = os_filename.to_str();
        if filename.is_none() {
            continue
        }

        if !filename.unwrap().ends_with(".idx") {
            continue
        }

        let entry_path = entry.path();
        let idx = match Index::from(std::fs::File::open(entry_path.clone())?) {
            Ok(xs) => xs,
            Err(_) => return Err(std::io::ErrorKind::InvalidData.into())
        };

        let mut epb = entry_path.to_path_buf();
        epb.set_extension("pack");

        let store = PackStore::new(move || {
            Ok(std::fs::File::open(epb.as_path()).expect("success?"))
        }, Some(idx));


        if let Ok(store) = store {
            stores.push(Box::new(move |id| {
                store.get(id, &|_id| {
                    Ok(None)
                })
            }));
        }
    }

    let get_id = |id: &Id| -> Result<Option<(Type, Box<std::io::Read>)>> {
        for store in &stores {
            let result = store(id)?;
            if result.is_some() {
                return Ok(result)
            }
        }

        return Ok(None)
    };

    let args: Vec<String> = std::env::args().collect();

    let mut id = match Id::from_str(&args[1]) {
        Some(xs) => xs,
        None => return Ok(())
    };

    loop {
        let found = match get_id(&id) {
            Err(_) => return Err(std::io::ErrorKind::InvalidData.into()),
            Ok(xs) => xs
        };

        if found.is_none() {
            println!("failed to find {}", id);
            return Err(std::io::ErrorKind::InvalidData.into());
        }

        let (ty, mut stream) = found.unwrap();
        let object = ty.load(&mut stream).expect("could not do the thing");

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
