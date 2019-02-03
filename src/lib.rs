#[macro_use]
extern crate error_chain;

mod crc;
pub mod id;
pub mod delta;
pub mod errors;
pub mod stores;
pub mod objects;
pub mod pack;
pub mod packindex;
pub mod refs;
pub mod walk;
pub mod identity;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
