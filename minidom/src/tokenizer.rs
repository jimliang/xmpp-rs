//! Streaming tokenizer (SAX parser)

use bytes::BytesMut;
use super::Token;

/// `Result::Err` type returned from `Tokenizer`
pub type TokenizerError = nom::error::Error<()>;

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
    pub fn pull(&mut self) -> Result<Option<Token>, TokenizerError> {
        /// cannot return an error with location info that points to
        /// our buffer that we still want to mutate
        fn erase_location<T>(e: nom::error::Error<T>) -> TokenizerError {
            nom::error::Error {
                input: (),
                code: e.code,
            }
        }
        
        let result: Option<(usize, Token)> = { match Token::parse(&self.buffer) {
            Ok((s, token)) =>
                Some((s.len(), token)),
            Result::Err(nom::Err::Incomplete(_)) =>
                None,
            Result::Err(nom::Err::Error(e)) =>
                return Err(erase_location(e)),
            Result::Err(nom::Err::Failure(e)) =>
                return Err(erase_location(e)),
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
                    name: "foo".to_owned(),
                    attrs: vec![("bar".to_owned(), "baz".to_owned())],
                    self_closing: false,
                },
                Token::Text("quux".to_owned()),
                Token::EndTag {
                    name: "foo".to_owned(),
                },
            ], run(chunk_size, buf));
        }
    }
}
