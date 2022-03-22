//! Parsed XML token

use nom::{
    branch::alt,
    bytes::streaming::{tag, take_while1},
    character::{is_space, streaming::{char, digit1, one_of, space0}},
    combinator::{not, peek, value},
    multi::many0,
    number::streaming::hex_u32,
    IResult,
};

/// Parsed XML token
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    /// XML element opening tag
    StartTag {
        /// Element name
        name: String,
        /// List of attributes
        attrs: Vec<(String, String)>,
        /// Is this tag self-closing (`/>`)?
        self_closing: bool,
    },
    /// XML element closing tag
    EndTag {
        /// Element name
        name: String,
    },
    /// Child text
    Text(String),
}

impl Token {
    /// Parse one token
    pub fn parse(s: &[u8]) -> IResult<&[u8], Token> {
        alt((
            Self::parse_tag,
            |s| {
                let (s, text) = Self::parse_text('<', s)?;
                Ok((s, Token::Text(text)))
            },
        ))(s)
    }

    fn parse_tag(s: &[u8]) -> IResult<&[u8], Token> {
        let (s, _) = tag("<")(s)?;
        alt((|s| -> IResult<&[u8], Token> {
            let (s, _) = tag("![CDATA[")(s)?;
            let mut end = None;
            for i in 0..s.len() - 2 {
                if &s[i..i + 3] == b"]]>" {
                    end = Some(i);
                    break
                }
            }
            if let Some(end) = end {
                let text = Self::str_from_utf8(&s[..end])?;
                Ok((&s[end + 3..], Token::Text(text.to_string())))
            } else {
                Err(nom::Err::Incomplete(nom::Needed::Unknown))
            }
        }, |s| {
            let (s, _) = tag("/")(s)?;
            let (s, _) = space0(s)?;
            let (s, name) = take_while1(|b| !(is_space(b) || b == b'>'))(s)?;
            let (s, _) = space0(s)?;
            let (s, _) = tag(">")(s)?;
            let name = Self::str_from_utf8(name)?;
            Ok((s, Token::EndTag { name: name.to_string() }))
        }, |s| {
            let (s, _) = space0(s)?;
            let (s, name) = take_while1(|b| !(is_space(b) || b == b'>' || b == b'/'))(s)?;
            let mut attrs = vec![];
            let mut self_closing = false;
            let mut s_ = s;
            loop {
                let (s, _) = space0(s_)?;
                let (s, attr) = alt((|s| {
                    let (s, _) = tag("/")(s)?;
                    let (s, _) = space0(s)?;
                    let (s, _) = tag(">")(s)?;
                    self_closing = true;
                    Ok((s, None))
                }, |s| {
                    let (s, _) = tag(">")(s)?;
                    Ok((s, None))
                }, |s| {
                    let (s, (name, value)) = Self::parse_attr(s)?;
                    Ok((s, Some((name, value))))
                }))(s)?;
                s_ = s;
                if let Some(attr) = attr {
                    attrs.push(attr);
                } else {
                    break;
                }
            }
            Ok((s_, Token::StartTag {
                name: Self::str_from_utf8(name)?
                    .to_owned(),
                attrs: attrs.into_iter()
                    .map(|(name, value)| (name.to_owned(), value.to_owned()))
                    .collect(),
                self_closing,
            }))
        }))(s)
    }

    fn parse_attr(s: &[u8]) -> IResult<&[u8], (&str, String)> {
        let (s, name) = take_while1(|b| !(is_space(b) || b == b'='))(s)?;
        let name = Self::str_from_utf8(name)?;
        let (s, _) = space0(s)?;
        let (s, _) = tag("=")(s)?;
        let (s, _) = space0(s)?;
        let (s, delim) = one_of("'\"")(s)?;
        let (s, value) = Self::parse_text(delim, s)?;
        let (s, _) = char(delim)(s)?;
        Ok((s, (name, value)))
    }

    fn parse_text(until: char, s: &[u8]) -> IResult<&[u8], String> {
        let (s, results) = many0(
            alt(
                (|s| {
                    let (s, _) = tag("&#")(s)?;
                    let (s, num) = digit1(s)?;
                    let (s, _) = char(';')(s)?;
                    let num: u32 = Self::str_from_utf8(num)?
                        .parse()
                        .map_err(|_| nom::Err::Failure(nom::error::Error::new(s, nom::error::ErrorKind::Fail)))?;
                    if let Some(c) = std::char::from_u32(num) {
                        Ok((s, format!("{}", c)))
                    } else {
                        Ok((s, format!("")))
                    }
                }, |s| {
                    let (s, _) = tag("&#x")(s)?;
                    let (s, num) = hex_u32(s)?;
                    let (s, _) = char(';')(s)?;
                    if let Some(c) = std::char::from_u32(num) {
                        Ok((s, format!("{}", c)))
                    } else {
                        Ok((s, format!("")))
                    }
                }, |s| {
                    let (s, _) = char('&')(s)?;
                    let (s, c) = alt((
                        value('&', tag("amp")),
                        value('<', tag("lt")),
                        value('>', tag("gt")),
                        value('"', tag("quot")),
                        value('\'', tag("apos")),
                    ))(s)?;
                    let (s, _) = char(';')(s)?;
                    Ok((s, format!("{}", c)))
                }, |s| {
                    let (s, _) = not(peek(char(until)))(s)?;
                    let (s, text) = take_while1(|b| b != until as u8 && b != b'&')(s)?;
                    let text = Self::str_from_utf8(text)?;
                    // TODO: CoW
                    Ok((s, text.to_owned()))
                })
            )
        )(s)?;

        let result = results.join("");
        Ok((s, result))
    }

    fn str_from_utf8(s: &[u8]) -> Result<&str, nom::Err<nom::error::Error<&[u8]>>> {
        std::str::from_utf8(s)
            .map_err(|_| nom::Err::Failure(nom::error::Error::new(s, nom::error::ErrorKind::Fail)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text() {
        assert_eq!(
            Ok((&b"</x"[..], Token::Text("foobar".to_string()))),
            Token::parse(b"foobar</x")
        );
    }

    #[test]
    fn test_text_entities() {
        assert_eq!(
            Ok((&b"</x"[..], Token::Text("\"<foo&bar>'".to_string()))),
            Token::parse(b"&quot;&lt;foo&amp;bar&gt;&apos;</x")
        );
    }

    #[test]
    fn test_text_entities_decimal() {
        assert_eq!(
            Ok((&b"</x"[..], Token::Text("foo\r\n".to_string()))),
            Token::parse(b"foo&#13;&#10;</x")
        );
    }

    #[test]
    fn test_text_entities_hexadecimal() {
        assert_eq!(
            Ok((&b"</x"[..], Token::Text("foo\r\n".to_string()))),
            Token::parse(b"foo&#xD;&#x0A;</x")
        );
    }

    #[test]
    fn test_cdata() {
        assert_eq!(
            Ok((&b""[..], Token::Text("<a href='>".to_string()))),
            Token::parse(b"<![CDATA[<a href='>]]>")
        );
    }

    #[test]
    fn test_tag() {
        assert_eq!(
            Ok((&b""[..], Token::StartTag {
                name: "foobar".to_string(),
                attrs: vec![],
                self_closing: false,
            })),
            Token::parse(b"<foobar>")
        );
    }

    #[test]
    fn test_attrs() {
        assert_eq!(
            Ok((&b""[..], Token::StartTag {
                name: "a".to_string(),
                attrs: vec![
                    ("a".to_owned(), "2'3".to_owned()),
                    ("b".to_owned(), "4\"2".to_owned()),
                    ("c".to_owned(), "".to_owned()),
                ],
                self_closing: false,
            })),
            Token::parse(b"<a a=\"2'3\" b = '4\"2' c = ''>")
        );
    }

    #[test]
    fn test_attrs_entities() {
        assert_eq!(
            Ok((&b""[..], Token::StartTag {
                name: "a".to_string(),
                attrs: vec![
                    ("a".to_owned(), "<3".to_owned()),
                ],
                self_closing: false,
            })),
            Token::parse(b"<a a='&lt;&#51;'>")
        );
    }

    #[test]
    fn test_self_closing_tag() {
        assert_eq!(
            Ok((&b""[..], Token::StartTag {
                name: "foobar".to_string(),
                attrs: vec![],
                self_closing: true,
            })),
            Token::parse(b"<foobar/>")
        );
    }

    #[test]
    fn test_end_tag() {
        assert_eq!(
            Ok((&b""[..], Token::EndTag {
                name: "foobar".to_string(),
            })),
            Token::parse(b"</foobar>")
        );
    }

    // TODO:
    // - DOCTYPE
    // - xmldecl
}
