//! SAX events to DOM tree conversion

use std::collections::BTreeMap;
use crate::Element;
use crate::prefixes::Prefixes;
use crate::token::{Attribute, LocalName, Token};

/// Tree-building parser state
pub struct TreeBuilder {
    /// Parsing stack
    stack: Vec<Element>,
    /// Namespace set stack by prefix
    prefixes_stack: Vec<Prefixes>,
    /// Document root element if finished
    pub root: Option<Element>,
}

impl TreeBuilder {
    /// Create a new one
    pub fn new() -> Self {
        TreeBuilder {
            stack: vec![],
            prefixes_stack: vec![],
            root: None,
        }
    }

    /// Stack depth
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Pop the top-most element from the stack
    pub fn pop(&mut self) -> Option<Element> {
        self.prefixes_stack.pop();
        self.stack.pop()
    }

    /// Lookup XML namespace declaration for given prefix (or no prefix)
    fn lookup_prefix(&self, prefix: &Option<String>) -> Option<&str> {
        for nss in self.prefixes_stack.iter().rev() {
            if let Some(ns) = nss.get(prefix) {
                return Some(ns);
            }
        }

        None
    }

    fn process_start_tag(&mut self, name: LocalName, attrs: Vec<Attribute>) {
        let mut prefixes = Prefixes::default();
        let mut attributes = BTreeMap::new();
        for attr in attrs.into_iter() {
            match (attr.name.prefix, attr.name.name) {
                (None, xmlns) if xmlns == "xmlns" => {
                    prefixes.insert(None, attr.value);
                }
                (Some(xmlns), prefix) if xmlns == "xmlns" => {
                    prefixes.insert(Some(prefix), attr.value);
                }
                (Some(prefix), name) => {
                    attributes.insert(format!("{}:{}", prefix, name), attr.value);
                }
                (None, name) => {
                    attributes.insert(name, attr.value);
                }
            }
        }
        self.prefixes_stack.push(prefixes.clone());

        let el = Element::new(
            name.name,
            self.lookup_prefix(&name.prefix).unwrap_or("").to_owned(),
            Some(name.prefix),
            prefixes,
            attributes,
            vec![]
        );
        self.stack.push(el);
    }

    fn process_end_tag(&mut self) {
        if let Some(el) = self.pop() {
            if self.depth() > 0 {
                let top = self.stack.len() - 1;
                self.stack[top].append_child(el);
            } else {
                self.root = Some(el);
            }
        }
    }

    fn process_text(&mut self, text: String) {
        if self.depth() > 0 {
            let top = self.stack.len() - 1;
            self.stack[top].append_text_node(text);
        }
    }

    /// Process a Token that you got out of a Tokenizer
    pub fn process_token(&mut self, token: Token) {
        match token {
            Token::XmlDecl { .. } => {},

            Token::StartTag {
                name,
                attrs,
                self_closing: false,
            } => self.process_start_tag(name, attrs),

            Token::StartTag {
                name,
                attrs,
                self_closing: true,
            } => {
                self.process_start_tag(name, attrs);
                self.process_end_tag();
            }

            Token::EndTag { .. } =>
                self.process_end_tag(),

            Token::Text(text) =>
                self.process_text(text),
        }
    }
}
