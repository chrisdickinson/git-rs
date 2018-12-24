#[macro_use]
extern crate error_chain;

pub mod id;
pub mod delta;
pub mod errors;
pub mod stores;
pub mod objects;
pub mod packindex;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
