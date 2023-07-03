use crate::parser::{Bound, Error, Parser, Result};
use log::debug;

#[derive(Debug, PartialEq, Eq)]
struct ByteSlice {
    pos: usize,
    len: usize,
}

impl ByteSlice {
    pub fn new(pos: usize, len: usize) -> ByteSlice {
        ByteSlice { pos, len }
    }

    pub fn bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        &data[self.pos..self.pos + self.len]
    }

    pub fn str<'a>(&self, data: &'a [u8]) -> Option<&'a str> {
        std::str::from_utf8(self.bytes(data)).ok()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Shard {
    Text(ByteSlice),
    Mention(ByteSlice),
    Hashtag(ByteSlice),
    Url(ByteSlice),
    //Invoice(Invoice)
    //Relay(String)
}

#[derive(Debug)]
struct Shards {
    shards: Vec<Shard>,
    num_words: i32,
}

impl Shards {
    fn new() -> Shards {
        Shards {
            shards: vec![],
            num_words: 0,
        }
    }

    fn parse_hashtag(parser: &mut Parser) -> Result<ByteSlice> {
        let start = parser.pos();
        match parser.parse_until(is_boundary_char) {
            Ok(()) | Err(Error::OutOfBounds(Bound::End)) => {
                debug!("got to hashtag boundary @ {}", parser.pos());
                let len = parser.pos() - start;
                if len <= 0 {
                    return Err(Error::NotFound);
                }
                return Ok(ByteSlice::new(start, len));
            }
            Err(err) => Err(err.into()),
        }
    }

    fn push_txt(&mut self, start: usize, upto: usize) {
        let len = upto - start;
        if len == 0 {
            return;
        }

        let txt_slice = ByteSlice::new(start, len);
        /*
        debug!(
            "pushing text block {:?} @ {} '{:?}'",
            txt_slice,
            parser.pos(),
            txt_slice.str(parser.data())
        );
        */
        self.shards.push(Shard::Text(txt_slice));
    }

    pub fn parse(content: &str) -> Result<Shards> {
        let mut parser = Parser::from_str(content);
        let len = parser.len();
        let mut shards = Shards::new();
        let mut start = parser.pos();

        while parser.pos() < len {
            let before_parse = parser.pos();
            let prev_boundary = is_left_boundary(&parser.peek_prev_byte());
            let c1 = parser.data()[parser.pos()] as char;
            parser.set_pos(parser.pos() + 1);

            if c1 == '#' && prev_boundary {
                match Shards::parse_hashtag(&mut parser) {
                    Ok(ht) => {
                        shards.push_txt(start, before_parse);
                        start = parser.pos();

                        debug!("pushing hashtag {:?}", ht);
                        shards.shards.push(Shard::Hashtag(ht));
                    }

                    Err(err) => {
                        debug!("failed parsing hashtag @ {}: {:?}", parser.pos(), err);
                    }
                }
            }
        }

        shards.push_txt(start, parser.pos());
        Ok(shards)
    }
}

fn is_boundary(r: &Result<char>) -> bool {
    match r {
        Err(Error::OutOfBounds(_)) => true,
        Err(_) => false,
        Ok(c) => is_boundary_char(*c),
    }
}

fn is_left_boundary(r: &Result<u8>) -> bool {
    match r {
        Err(Error::OutOfBounds(_)) => true,
        Err(_) => false,
        Ok(c) => is_left_boundary_char(*c),
    }
}

fn is_eof<T>(r: &Result<T>) -> bool {
    match r {
        Err(Error::OutOfBounds(Bound::End)) => true,
        _ => false,
    }
}

fn is_boundary_char(c: char) -> bool {
    c.is_ascii_whitespace() || c.is_ascii_punctuation()
}

fn is_left_boundary_char(c: u8) -> bool {
    is_boundary_char(c as char) || ((c & 0b10000000) == 0b10000000)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Setup function that is only run once, even if called multiple times.
    fn setup() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    #[test]
    fn test_is_boundary() {
        setup();

        let content = "a";
        let parser = Parser::from_str(&content);
        let res = parser.peek_prev_char();
        assert_eq!(is_boundary(&res), true);
    }

    #[test]
    fn test_parse_hashtag_basic() {
        setup();

        let content = "abc #😎";
        debug!("hashtag_basic content '{}'", content);
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0], Shard::Text(ByteSlice::new(0, 4)));
        assert_eq!(bs[1], Shard::Hashtag(ByteSlice::new(5, 4)));
    }

    #[test]
    fn test_parse_hashtag_adjacent() {
        setup();

        let content = "aa#abc";
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 1);
        assert_eq!(bs[0], Shard::Text(ByteSlice::new(0, 6)));
    }

    #[test]
    fn test_parse_hashtag_start() {
        setup();

        let content = "#abc.";
        debug!("test_parse_hashtag_start '{}'", content);
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0], Shard::Hashtag(ByteSlice::new(1, 3)));
        assert_eq!(bs[1], Shard::Text(ByteSlice::new(4, 1)));
    }

    #[test]
    fn test_parse_hashtag_end() {
        setup();

        let content = "#abc";
        debug!("test_parse_hashtag_end '{}'", content);
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 1);
        assert_eq!(bs[0], Shard::Hashtag(ByteSlice::new(1, 3)));
    }

    #[test]
    fn test_parse_hashtag_punc_before() {
        setup();

        let content = ".#abc";
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0], Shard::Text(ByteSlice::new(0, 1)));
        assert_eq!(bs[1], Shard::Hashtag(ByteSlice::new(2, 3)));
    }

    #[test]
    fn test_multiple_hashtags() {
        setup();

        let content = ".#alice.#bob";
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 4);
        assert_eq!(bs[0], Shard::Text(ByteSlice::new(0, 1)));
        assert_eq!(bs[1], Shard::Hashtag(ByteSlice::new(2, 5)));
        assert_eq!(bs[2], Shard::Text(ByteSlice::new(7, 1)));
        assert_eq!(bs[3], Shard::Hashtag(ByteSlice::new(9, 3)));
    }

    #[test]
    fn test_multiple_adjacent_hashtags() {
        setup();

        let content = "#alice#bob";
        debug!("test_multiple_adjacent_hashtags '{}'", content);
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0], Shard::Hashtag(ByteSlice::new(1, 5)));
        assert_eq!(bs[1], Shard::Hashtag(ByteSlice::new(7, 3)));
    }

    #[test]
    fn test_parse_hashtag_emoji_before() {
        setup();

        // 00000000: f09f 98a4 2361 6263    ....#abc
        let content = "😤#abc";
        let shards = Shards::parse(content).unwrap();
        let bs = shards.shards;
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0], Shard::Text(ByteSlice::new(0, 4)));
        assert_eq!(bs[1], Shard::Hashtag(ByteSlice::new(5, 3)));
    }
}
