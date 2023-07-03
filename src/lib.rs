//! This crate is a fast, zero-copy [nostr] content parser. What is a
//! [nostr] content parser? There can be many elements within the body of
//! a note on [nostr], and to render them properly they need to be parsed
//! out. shatter will find all the locations of the elements of interest
//! and mark them so that can pick them out and render them properly.
//!
//! [nostr]: https://github.com/nostr-protocol/nostr
pub mod shard;
