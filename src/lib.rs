#[macro_use]
extern crate error_chain;

pub mod id;
pub mod errors;
pub mod objects;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
