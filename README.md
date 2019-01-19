# git-rs

Implementing git in rust for fun and education!

This is actually my second stab at it, so big blocks will land in place from my
first attempt. I'm trying again this year after reading more of "Programming
Rust" (Blandy, Orendorff).

## TODO

- [x] Read objects from loose store
- [x] Read objects from pack store
    - [x] Read packfile indexes
    - [x] Read delta'd objects
    - [x] Fix interface so we don't need to run `open` for each `read()`
- [x] Load refs off of disk
- [ ] Load packed-refs
- [x] Parse git signatures ("Identity"'s)
- [x] Create iterator for walking commit graph
- [x] Create iterator for walking trees
    - [ ] Materialize trees to disk (post gitindex?)
- [ ] Create index from packfile
- [ ] Create interface for writing new objects
- [ ] `.git/index` support
    - [ ] Read git index cache
    - [ ] Write git index cache
- [ ] Create packfile from list of objects (API TKTK)
- [ ] Network protocol
    - [ ] receive-pack
    - [ ] send-pack
- [ ] Try publishing to crates
    - [ ] Write documentation
    - [ ] Use crate in another project

* * *

## PLAN

### 2018-01-15 Update

- I added an (experimental) `git_rs::walk::tree` iterator to take a Tree and yield
  a path + a blob for each item.
    - It's probably slower than it should be: for each item it has to clone a `PathBuf`, because I couldn't work out the lifetimes.
    - **If you know how to fix that**, please [open an issue][ref_8] and let me know :revolving_hearts:
- I took some time to clean up the warnings during builds.
    - Oh! I also installed [Clippy][ref_9] which warns about higher level antipatterns in Rust!
- I'm still noodling over the **2-3x** slowdown between vanilla git and Our Git.
    - I think I might create two packfile interfaces -- one "generic" and one "mmap"'d, to see if
      one or the other makes up the difference in performance.
        - This also has the virtue of being `unsafe` code, which is something I have not yet used
          in Rust!

* * *

### 2019-01-06 Update

- I wrote an iterator for commits! The [first cut][ref_6] kept a `Vec` of `(Id, Commit)` around,
  so we could always pick the most recent "next" commit out of the graph (since commits may have
  many parents.)
    - But in finishing up the collections section of "Programming Rust" I noticed that `BinaryHeap`
      was available, which keeps entries in sorted order. You don't often get to choose the underlying
      storage mechanism of your collections in JS, so this hadn't occurred to me!
    - Anyway. I swapped out the `Vec` for a `BinaryHeap` [in this commit][ref_7]. Because this pushes
      the ordering into an `Ord` impl for a type, this opens up the possibility of using the one iterator
      definition for multiple different orderings. Neat!
- Testing against a couple long-lived repo, the results coming out of `git_rs` are exactly the same as
  `git`!
    - However, it takes about **twice** the time: **60ms** for `git_rs` where `git` takes **30ms**.
    - I think I have a lead on this, and it has to do with packfile stores: each read from a packfile
      opens a new `File` instance.
- I've added a **TODO** section to keep track of what comes next!

* * *

### 2019-01-02 Update

- I implemented [ref loading][ref_2]. It was a bit of a pain! Translating to and
  from `Path` types took a bit of doing.
- I've been trying to read up on Rust idioms -- I found a couple of resources:
    - [The Rust API Guidelines][ref_3] doc has been _very_ helpful.
    - **@mre**'s [idiomatic rust repo][ref_4] collects many interesting links.
    - I've also been idly checking out [videos from RustConf 2018][ref_5]
- As a result, I've implemented `FromStr` for `Id`, (hopefully) giving it a
  more idiomatic API -- `let id: Id = str.parse()?`

* * *

### 2018-12-27 Update

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

* * *

### 2018-12-21 Update

- Decided to focus on moving a bit slower and making sure I have tests for
  primitives this time around.
- Moved away from my original `Box<Write>` trait object design for object
  instance reading & storage format in favor of generics.

* * *

[ref_0]: https://github.com/chrisdickinson/git-rs/blob/fdbe4ac7c781a5c085777baafbd15655be2eca0b/src/objects/commit.rs#L20-L30
[ref_1]: https://github.com/chrisdickinson/git-rs/blob/fdbe4ac7c781a5c085777baafbd15655be2eca0b/src/packindex.rs#L116-L126
[ref_2]: https://github.com/chrisdickinson/git-rs/commit/6157317fb18acac0633c624e9831282a950b4db0
[ref_3]: https://rust-lang-nursery.github.io/api-guidelines/
[ref_4]: https://github.com/mre/idiomatic-rust
[ref_5]: https://www.youtube.com/playlist?list=PL85XCvVPmGQi3tivxDDF1hrT9qr5hdMBZ
[ref_6]: https://github.com/chrisdickinson/git-rs/blob/254d97e3d840eded4e5ff5a06b9414ff9396e976/src/walk/commits.rs#L56-L71
[ref_7]: https://github.com/chrisdickinson/git-rs/commit/f8f4cf5f1430b14d3ef0b298ffa9f2cd880d5c28/src/walk/commits.rs#L40
[ref_8]: https://github.com/chrisdickinson/git-rs/issues/new?title=Here%27s%20how%20to%20remove%20the%20clone()%20from%20walk::tree
[ref_9]: https://github.com/rust-lang/rust-clippy
