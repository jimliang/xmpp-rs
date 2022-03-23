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

/// Attribute name with prefix
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalName {
    /// Element/attribute prefix
    pub prefix: Option<String>,
    /// Element/attribute name
    pub name: String,
}

impl From<&str> for LocalName {
    fn from(s: &str) -> Self {
        match s.split_once(':') {
            Some((prefix, name)) =>
                LocalName {
                    prefix: Some(prefix.to_owned()),
                    name: name.to_owned(),
                },
            None =>
                LocalName {
                    prefix: None,
                    name: s.to_owned(),
                },
        }
    }
}

/// Name-value pair of an element's attribute
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Attribute {
    /// Attribute name
    pub name: LocalName,
    /// Attribute value
    pub value: String,
}

/// Parsed XML token
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    /// XML element opening tag
    StartTag {
        /// Element name
        name: LocalName,
        /// List of attributes
        attrs: Vec<Attribute>,
        /// Is this tag self-closing (`/>`)?
        self_closing: bool,
    },
    /// XML element closing tag
    EndTag {
        /// Element name
        name: LocalName,
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
            // CDATA
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
            // EndTag
            let (s, _) = tag("/")(s)?;
            let (s, _) = space0(s)?;
            let (s, name) = take_while1(|b| !(is_space(b) || b == b'>'))(s)?;
            let (s, _) = space0(s)?;
            let (s, _) = tag(">")(s)?;
            let name = Self::str_from_utf8(name)?;
            Ok((s, Token::EndTag { name: name.into() }))
        }, |s| {
            // StartTag
            let (s, _) = space0(s)?;
            let (s, name) = take_while1(|b| !(is_space(b) || b == b'>' || b == b'/'))(s)?;
            let (s, _) = space0(s)?;
            let (s, attrs) = many0(|s| {
                let (s, (name, value)) = Self::parse_attr(s)?;
                let (s, _) = space0(s)?;
                Ok((s, (name, value)))
            })(s)?;

            let (s, self_closing) = alt((|s| {
                let (s, _) = tag("/")(s)?;
                let (s, _) = space0(s)?;
                let (s, _) = tag(">")(s)?;
                Ok((s, true))
            }, |s| {
                let (s, _) = tag(">")(s)?;
                Ok((s, false))
            }))(s)?;

            Ok((s, Token::StartTag {
                name: Self::str_from_utf8(name)?
                    .into(),
                attrs: attrs.into_iter()
                    .map(|(name, value)| Attribute { name: name.into(), value })
                    .collect(),
                self_closing,
            }))
        }))(s)
    }

    fn parse_attr(s: &[u8]) -> IResult<&[u8], (&str, String)> {
        let (s, name) = take_while1(|b| !(is_space(b) || b == b'=' || b == b'/' || b == b'>'))(s)?;
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

    fn attr(name: &str, value: &str) -> Attribute {
        Attribute {
            name: name.into(),
            value: value.to_owned(),
        }
    }

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
                name: "foobar".into(),
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
                name: "a".into(),
                attrs: vec![
                    attr("a", "2'3"),
                    attr("b", "4\"2"),
                    attr("c", ""),
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
                name: "a".into(),
                attrs: vec![
                    attr("a", "<3"),
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
                name: "foobar".into(),
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
                name: "foobar".into(),
            })),
            Token::parse(b"</foobar>")
        );
    }

    #[test]
    fn test_element_prefix() {
        assert_eq!(
            Ok((&b""[..], Token::StartTag {
                name: LocalName {
                    name: "z".to_owned(),
                    prefix: Some("x".to_owned()),
                },
                attrs: vec![],
                self_closing: true,
            })),
            Token::parse(b"<x:z/>")
        );
    }

    #[test]
    fn test_attr_prefix() {
        assert_eq!(
            Ok((&b""[..], Token::StartTag {
                name: "a".into(),
                attrs: vec![Attribute {
                    name: LocalName {
                        name: "abc".to_owned(),
                        prefix: Some("xyz".to_owned()),
                    },
                    value: "".to_owned(),
                }],
                self_closing: false,
            })),
            Token::parse(b"<a xyz:abc=''>")
        );
    }

    // TODO:
    // - DOCTYPE
    // - xmldecl
}
