#[macro_use]
extern crate error_chain;

pub mod delta;
pub mod errors;
pub mod id;
pub mod identity;
pub mod objects;
pub mod pack;
pub mod refs;
pub mod stores;
pub mod walk;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
