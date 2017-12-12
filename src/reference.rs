use std;
use regex::Regex;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::collections::HashMap;

use id::Id;

#[derive(Debug)]
pub struct Ref {
    _id: Id,
}

impl Ref {
    pub fn new(path: &str) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut contents = String::new();

        let written = file.read_to_string(&mut contents)?;
        let id = Id::from(&contents).expect("failed to read ID");

        Ok(Ref { _id: id })
    }

    pub fn from_packed(path: &str, map: &mut HashMap<String, Ref>) -> Result<(), std::io::Error> {
        lazy_static! {
            static ref comment_re: Regex = Regex::new("#.*$").unwrap();
            static ref peeled_re: Regex = Regex::new(r"^\^").unwrap();
        }


        let mut file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut refname = String::new();

        for next_line in reader.lines() {
            let raw_line = next_line?;
            let comments_stripped = comment_re.replace_all(&raw_line, "");
            let line = comments_stripped.trim();
            if line.len() == 0 {
                continue;
            }

            if peeled_re.is_match(line) {
                let id = match Id::from(&line[1..]) {
                    Err(_) => continue,
                    Ok(xs) => xs
                };
                map.insert(refname.clone(), Ref { _id: id });
            } else if let Some(space_idx) = line.find(' ') {
                let id = match Id::from(&line[0..space_idx]) {
                    Err(_) => continue,
                    Ok(xs) => xs
                };
                refname = String::from(&line[space_idx + 1..]);
                map.insert(refname.clone(), Ref { _id: id });
            }
        }

        Ok(())
    }

    pub fn to_id (&self) -> Id {
        Id::clone(&self._id)
    }
}
