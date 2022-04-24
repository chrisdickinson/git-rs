#[macro_use]
extern crate error_chain;

pub mod id;
pub mod delta;
pub mod errors;
pub mod stores;
pub mod objects;
pub mod pack;
pub mod refs;
pub mod walk;
pub mod human_metadata;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
