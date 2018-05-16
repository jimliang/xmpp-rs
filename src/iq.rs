// Copyright (c) 2017 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
// Copyright (c) 2017 Maxime “pep” Buquet <pep+code@bouah.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use try_from::TryFrom;

use minidom::Element;
use minidom::IntoAttributeValue;

use jid::Jid;

use error::Error;

use ns;

use stanza_error::StanzaError;

/// Should be implemented on every known payload of an `<iq type='get'/>`.
pub trait IqGetPayload: TryFrom<Element> + Into<Element> {}

/// Should be implemented on every known payload of an `<iq type='set'/>`.
pub trait IqSetPayload: TryFrom<Element> + Into<Element> {}

/// Should be implemented on every known payload of an `<iq type='result'/>`.
pub trait IqResultPayload: TryFrom<Element> + Into<Element> {}

#[derive(Debug, Clone)]
pub enum IqType {
    Get(Element),
    Set(Element),
    Result(Option<Element>),
    Error(StanzaError),
}

impl<'a> IntoAttributeValue for &'a IqType {
    fn into_attribute_value(self) -> Option<String> {
        Some(match *self {
            IqType::Get(_) => "get",
            IqType::Set(_) => "set",
            IqType::Result(_) => "result",
            IqType::Error(_) => "error",
        }.to_owned())
    }
}

/// The main structure representing the `<iq/>` stanza.
#[derive(Debug, Clone)]
pub struct Iq {
    pub from: Option<Jid>,
    pub to: Option<Jid>,
    pub id: Option<String>,
    pub payload: IqType,
}

impl TryFrom<Element> for Iq {
    type Err = Error;

    fn try_from(root: Element) -> Result<Iq, Error> {
        check_self!(root, "iq", DEFAULT_NS);
        let from = get_attr!(root, "from", optional);
        let to = get_attr!(root, "to", optional);
        let id = get_attr!(root, "id", optional);
        let type_: String = get_attr!(root, "type", required);

        let mut payload = None;
        let mut error_payload = None;
        for elem in root.children() {
            if payload.is_some() {
                return Err(Error::ParseError("Wrong number of children in iq element."));
            }
            if type_ == "error" {
                if elem.is("error", ns::DEFAULT_NS) {
                    if error_payload.is_some() {
                        return Err(Error::ParseError("Wrong number of children in iq element."));
                    }
                    error_payload = Some(StanzaError::try_from(elem.clone())?);
                } else if root.children().count() != 2 {
                    return Err(Error::ParseError("Wrong number of children in iq element."));
                }
            } else {
                payload = Some(elem.clone());
            }
        }

        let type_ = if type_ == "get" {
            if let Some(payload) = payload {
                IqType::Get(payload)
            } else {
                return Err(Error::ParseError("Wrong number of children in iq element."));
            }
        } else if type_ == "set" {
            if let Some(payload) = payload {
                IqType::Set(payload)
            } else {
                return Err(Error::ParseError("Wrong number of children in iq element."));
            }
        } else if type_ == "result" {
            if let Some(payload) = payload {
                IqType::Result(Some(payload))
            } else {
                IqType::Result(None)
            }
        } else if type_ == "error" {
            if let Some(payload) = error_payload {
                IqType::Error(payload)
            } else {
                return Err(Error::ParseError("Wrong number of children in iq element."));
            }
        } else {
            return Err(Error::ParseError("Unknown iq type."));
        };

        Ok(Iq {
            from: from,
            to: to,
            id: id,
            payload: type_,
        })
    }
}

impl From<Iq> for Element {
    fn from(iq: Iq) -> Element {
        let mut stanza = Element::builder("iq")
                                 .ns(ns::DEFAULT_NS)
                                 .attr("from", iq.from)
                                 .attr("to", iq.to)
                                 .attr("id", iq.id)
                                 .attr("type", &iq.payload)
                                 .build();
        let elem = match iq.payload {
            IqType::Get(elem)
          | IqType::Set(elem)
          | IqType::Result(Some(elem)) => elem,
            IqType::Error(error) => error.into(),
            IqType::Result(None) => return stanza,
        };
        stanza.append_child(elem);
        stanza
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stanza_error::{ErrorType, DefinedCondition};
    use compare_elements::NamespaceAwareCompare;
    use disco::DiscoInfoQuery;

    #[test]
    fn test_require_type() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client'/>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept'/>".parse().unwrap();
        let error = Iq::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'type' missing.");
    }

    #[test]
    fn test_get() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='get'>
            <foo xmlns='bar'/>
        </iq>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='get'>
            <foo xmlns='bar'/>
        </iq>".parse().unwrap();
        let iq = Iq::try_from(elem).unwrap();
        let query: Element = "<foo xmlns='bar'/>".parse().unwrap();
        assert_eq!(iq.from, None);
        assert_eq!(iq.to, None);
        assert_eq!(iq.id, None);
        assert!(match iq.payload {
            IqType::Get(element) => element.compare_to(&query),
            _ => false
        });
    }

    #[test]
    fn test_set() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='set'>
            <vCard xmlns='vcard-temp'/>
        </iq>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='set'>
            <vCard xmlns='vcard-temp'/>
        </iq>".parse().unwrap();
        let iq = Iq::try_from(elem).unwrap();
        let vcard: Element = "<vCard xmlns='vcard-temp'/>".parse().unwrap();
        assert_eq!(iq.from, None);
        assert_eq!(iq.to, None);
        assert_eq!(iq.id, None);
        assert!(match iq.payload {
            IqType::Set(element) => element.compare_to(&vcard),
            _ => false
        });
    }

    #[test]
    fn test_result_empty() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='result'/>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='result'/>".parse().unwrap();
        let iq = Iq::try_from(elem).unwrap();
        assert_eq!(iq.from, None);
        assert_eq!(iq.to, None);
        assert_eq!(iq.id, None);
        assert!(match iq.payload {
            IqType::Result(None) => true,
            _ => false,
        });
    }

    #[test]
    fn test_result() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='result'>
            <query xmlns='http://jabber.org/protocol/disco#items'/>
        </iq>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='result'>
            <query xmlns='http://jabber.org/protocol/disco#items'/>
        </iq>".parse().unwrap();
        let iq = Iq::try_from(elem).unwrap();
        let query: Element = "<query xmlns='http://jabber.org/protocol/disco#items'/>".parse().unwrap();
        assert_eq!(iq.from, None);
        assert_eq!(iq.to, None);
        assert_eq!(iq.id, None);
        assert!(match iq.payload {
            IqType::Result(Some(element)) => element.compare_to(&query),
            _ => false,
        });
    }

    #[test]
    fn test_error() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='error'>
            <ping xmlns='urn:xmpp:ping'/>
            <error type='cancel'>
                <service-unavailable xmlns='urn:ietf:params:xml:ns:xmpp-stanzas'/>
            </error>
        </iq>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='error'>
            <ping xmlns='urn:xmpp:ping'/>
            <error type='cancel'>
                <service-unavailable xmlns='urn:ietf:params:xml:ns:xmpp-stanzas'/>
            </error>
        </iq>".parse().unwrap();
        let iq = Iq::try_from(elem).unwrap();
        assert_eq!(iq.from, None);
        assert_eq!(iq.to, None);
        assert_eq!(iq.id, None);
        match iq.payload {
            IqType::Error(error) => {
                assert_eq!(error.type_, ErrorType::Cancel);
                assert_eq!(error.by, None);
                assert_eq!(error.defined_condition, DefinedCondition::ServiceUnavailable);
                assert_eq!(error.texts.len(), 0);
                assert_eq!(error.other, None);
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_children_invalid() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='error'></iq>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='error'></iq>".parse().unwrap();
        let error = Iq::try_from(elem).unwrap_err();
        let message = match error {
            Error::ParseError(string) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Wrong number of children in iq element.");
    }

    #[test]
    fn test_serialise() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='result'/>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='result'/>".parse().unwrap();
        let iq2 = Iq {
            from: None,
            to: None,
            id: None,
            payload: IqType::Result(None),
        };
        let elem2 = iq2.into();
        assert_eq!(elem, elem2);
    }

    #[test]
    fn test_disco() {
        #[cfg(not(feature = "component"))]
        let elem: Element = "<iq xmlns='jabber:client' type='get'><query xmlns='http://jabber.org/protocol/disco#info'/></iq>".parse().unwrap();
        #[cfg(feature = "component")]
        let elem: Element = "<iq xmlns='jabber:component:accept' type='get'><query xmlns='http://jabber.org/protocol/disco#info'/></iq>".parse().unwrap();
        let iq = Iq::try_from(elem).unwrap();
        let disco_info = match iq.payload {
            IqType::Get(payload) => DiscoInfoQuery::try_from(payload).unwrap(),
            _ => panic!(),
        };
        assert!(disco_info.node.is_none());
    }
}
