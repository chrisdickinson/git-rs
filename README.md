# git-rs

Implementing git in rust for fun and education!

This is actually my second stab at it, so big blocks will land in place from my
first attempt. I'm trying again this year after reading more of "Programming
Rust" (Blandy, Orendorff).

## 2018-01-02 Update

- I implemented [ref loading][ref_2]. It was a bit of a pain! Translating to and
  from `Path` types took a bit of doing.
- I've been trying to read up on Rust idioms -- I found a couple of resources:
    - [The Rust API Guidelines][ref_3] doc has been _very_ helpful.
    - **@mre**'s [idiomatic rust repo][ref_4] collects many interesting links.
    - I've also been idly checking out [videos from RustConf 2018][ref_5]
- As a result, I've implemented `FromStr` for `Id`, (hopefully) giving it a
  more idiomatic API -- `let id: Id = str.parse()?`

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

## 2018-12-21 Update

- Decided to focus on moving a bit slower and making sure I have tests for
  primitives this time around.
- Moved away from my original `Box<Write>` trait object design for object
  instance reading & storage format in favor of generics.

[ref_0]: https://github.com/chrisdickinson/git-rs/blob/fdbe4ac7c781a5c085777baafbd15655be2eca0b/src/objects/commit.rs#L20-L30
[ref_1]: https://github.com/chrisdickinson/git-rs/blob/fdbe4ac7c781a5c085777baafbd15655be2eca0b/src/packindex.rs#L116-L126
[ref_2]: https://github.com/chrisdickinson/git-rs/commit/6157317fb18acac0633c624e9831282a950b4db0
[ref_3]: https://rust-lang-nursery.github.io/api-guidelines/
[ref_4]: https://github.com/mre/idiomatic-rust
[ref_5]: https://www.youtube.com/playlist?list=PL85XCvVPmGQi3tivxDDF1hrT9qr5hdMBZ
