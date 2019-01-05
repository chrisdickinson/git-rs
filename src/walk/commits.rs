use std::collections::{ HashMap, HashSet };

use crate::objects::commit::Commit;
use crate::stores::StorageSet;
use crate::objects::Object;
use crate::id::Id;

pub struct CommitIterator<'a> {
    storage_set: &'a StorageSet,
    seen: HashSet<Id>,
    target: Vec<(Id, Commit)>
}

impl<'a> CommitIterator<'a> {
    pub fn new(storage_set: &'a StorageSet, id: &Id, seen: Option<HashSet<Id>>) -> CommitIterator<'a> {
        let mut seen = seen.unwrap_or_else(|| HashSet::<Id>::new());

        let first = storage_set.get_and_load(id).ok()
            .unwrap_or(None);

        let target = if let Some(xs) = first {
            if let Object::Commit(head) = xs {
                vec![(id.clone(), head)]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        seen.insert(id.clone());
        CommitIterator {
            target,
            storage_set,
            seen,
        }
    }
}

impl<'a> Iterator for CommitIterator<'a> {
    type Item = (Id, Commit);

    fn next(&mut self) -> Option<Self::Item> {
        // okay, so given a set of commits:
        // if the set is empty, return None
        //      take() the latest commit.
        //      get the parents of the latest commit.
        //          remove all seen commits from the parents.
        //          add the remaining parent ids to seen.
        //          push remaining parent commits into the vector.

        if self.target.len() == 0 {
            return None
        }

        let mut newest = &self.target[0].1;
        let mut newest_idx = 0;
        for (idx, (ref id, ref commit)) in self.target.iter().enumerate().skip(1) {
            if let Some(ref rhs) = commit.committer() {
                if let Some(ref lhs) = newest.committer() {
                    if lhs.at() < rhs.at() {
                        newest = commit;
                        newest_idx = idx;
                    }
                }
            }
        }

        let options = self.target.iter().map(|(ref id, _)| {
            id.to_string()
        }).collect::<Vec<String>>().join(", ");

        let mut parents: Vec<(Id, Commit)> = match newest.parents() {
            Some(xs) => xs.iter().filter_map(|id| {
                if self.seen.contains(id) {
                    return None
                }

                if let Object::Commit(commit) = self.storage_set.get_and_load(id).ok()?? {
                    self.seen.insert(id.clone());
                    return Some((id.clone(), commit))
                } else {
                    return None
                }
            }).collect(),
            None => Vec::new()
        };

        if parents.len() > 0 {
            let idstr = self.target[newest_idx].0.to_string();
            let first_parent = parents.pop().unwrap();
            let replaced = std::mem::replace(&mut self.target[newest_idx], first_parent);

            if parents.len() > 0 {
                self.target.append(&mut parents);
            }

            return Some(replaced);
        } else {
            return Some(self.target.remove(newest_idx))
        }
    }
}
