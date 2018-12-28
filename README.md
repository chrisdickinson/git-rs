# git-rs

Implementing git in rust for fun and education!

This is actually my second stab at it, so big blocks will land in place from my
first attempt. I'm trying again this year after reading more of "Programming
Rust" (Blandy, Orendorff).

## 2018-12-27 Update

- Rust is feeling more natural. [This chain][ref_0] felt natural to write. I
  was even able to [cross-index a list][ref_1] with only a minimum of fighting
  the borrow checker.
- I split the objects interface into Type + boxed read with a method for reifying
  the data into an Object. This feels good! It lets folks check to see, for example,
  if they're referring to a Blob without having to load the entire Blob into memory.
- The closure interface for the loose interface works pretty well, but pack interfaces
  need to be able to ask the overarching set of stores for a reference due to REF_DELTA
  objects. This is a bummer, because it really quickly turns into "fighting the borrow
  checker." Right now I think the way forward is to build a StorageSet that holds a Vec
  of heterogenous `Box<Storage>` objects, where `Storage` is a new trait that specifies
  `get(&Id, &StorageSet)`.
    - A sidenote re: the loose store: it feels kind of odd to have to produce a
      `git_rs::Error` instead of a `std::io::Error`. Room for improvement!
- Oh! It was pretty easy to add a binary to this lib crate. And now we can `git log`
  other repos!

[ref_0]: https://github.com/chrisdickinson/git-rs/blob/fdbe4ac7c781a5c085777baafbd15655be2eca0b/src/objects/commit.rs#L20-L30
[ref_1]: https://github.com/chrisdickinson/git-rs/blob/fdbe4ac7c781a5c085777baafbd15655be2eca0b/src/packindex.rs#L116-L126

## 2018-12-21 Update

- Decided to focus on moving a bit slower and making sure I have tests for
  primitives this time around.
- Moved away from my original `Box<Write>` trait object design for object
  instance reading & storage format in favor of generics.
