// Copyright (c) 2022 Astro <astro@spaceboyz.net>

//! Streaming tokenizer (SAX parser)

use bytes::BytesMut;
use super::{Error, Token};

/// `Result::Err` type returned from `Tokenizer`
pub type TokenizerError = nom::error::Error<String>;

/// Streaming tokenizer (SAX parser)
pub struct Tokenizer {
    buffer: BytesMut,
}

impl Tokenizer {
    /// Construct a new tokenizer
    pub fn new() -> Self {
        Tokenizer {
            buffer: BytesMut::new(),
        }
    }

    /// Add content to the inner buffer
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Is the internal buffer empty?
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Parse the next document fragment
    pub fn pull(&mut self) -> Result<Option<Token>, Error> {
        /// cannot return an error with location info that points to
        /// our buffer that we still want to mutate
        fn with_input_to_owned(e: nom::error::Error<&[u8]>) -> TokenizerError {
            nom::error::Error {
                input: std::str::from_utf8(e.input)
                    .unwrap_or("invalud UTF-8")
                    .to_owned(),
                code: e.code,
            }
        }
        
        let result: Option<(usize, Token)> = { match Token::parse(&self.buffer) {
            Ok((s, token)) =>
                Some((s.len(), token)),
            Result::Err(nom::Err::Incomplete(_)) =>
                None,
            Result::Err(nom::Err::Error(e)) =>
                return Err(with_input_to_owned(e).into()),
            Result::Err(nom::Err::Failure(e)) =>
                return Err(with_input_to_owned(e).into()),
        } };
        match result {
           Some((s_len, token)) => {
               let _ = self.buffer.split_to(self.buffer.len() - s_len);
               Ok(Some(token))
           }
            None => Ok(None)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Attribute;

    #[test]
    fn test() {
        fn run(chunk_size: usize, buf: &[u8]) -> Vec<Token> {
            let mut tokenizer = Tokenizer::new();
            let mut tokens = vec![];

            let mut pos = 0;
            while pos < buf.len() {
                tokenizer.push(&buf[pos..(pos + chunk_size).min(buf.len())]);
                pos += chunk_size;

                while let Some(token) = tokenizer.pull().unwrap() {
                    tokens.push(token)
                }
            }

            tokens
        }

        let buf = b"<foo bar='baz'>quux</foo>";
        for chunk_size in 1..=buf.len() {
            assert_eq!(vec![
                Token::StartTag {
                    name: "foo".into(),
                    attrs: vec![Attribute {
                        name: "bar".into(),
                        value: "baz".to_owned(),
                    }],
                    self_closing: false,
                },
                Token::Text("quux".to_owned()),
                Token::EndTag {
                    name: "foo".into(),
                },
            ], run(chunk_size, buf));
        }
    }
}
