# git-rs

Implementing git in rust for fun and education!

This is actually my second stab at it, so big blocks will land in place from my
first attempt. I'm trying again this year after reading more of "Programming
Rust" (Blandy, Orendorff).

## 2018-12-21 Update

- Decided to focus on moving a bit slower and making sure I have tests for
  primitives this time around.
- Moved away from my original `Box<Write>` trait object design for object
  instance reading & storage format in favor of generics.
