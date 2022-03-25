// Copyright (c) 2022 Astro <astro@spaceboyz.net>

//! SAX events to DOM tree conversion

use std::collections::{BTreeMap, HashMap};
use rxml::{CData, Event, QName};
use crate::{Element, Error};
use crate::prefixes::Prefixes;

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

    /// Get the top-most element from the stack but don't remove it
    pub fn top(&mut self) -> Option<&Element> {
        self.stack.last()
    }

    /// Pop the top-most element from the stack
    fn pop(&mut self) -> Option<Element> {
        self.prefixes_stack.pop();
        self.stack.pop()
    }

    /// Unshift the first child of the top element
    pub fn unshift_child(&mut self) -> Option<Element> {
        let depth = self.stack.len();
        if depth > 0 {
            self.stack[depth - 1].unshift_child()
        } else {
            None
        }
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

    fn process_start_tag(&mut self, (prefix, name): QName, attrs: HashMap<QName, CData>) -> Result<(), Error> {
        dbg!(&attrs);
        let mut prefixes = Prefixes::default();
        let mut attributes = BTreeMap::new();
        for ((prefix, name), value) in attrs.into_iter() {
            match (prefix, name) {
                (None, xmlns) if xmlns == "xmlns" => {
                    prefixes.insert(None, value);
                }
                (Some(xmlns), prefix) if *xmlns == "xmlns" => {
                    prefixes.insert(Some(prefix.as_string()), value);
                }
                (Some(prefix), name) => {
                    attributes.insert(format!("{}:{}", prefix, name), value.as_string());
                }
                (None, name) => {
                    attributes.insert(name.as_string(), value.as_string());
                }
            }
        }
        dbg!(&prefixes);
        self.prefixes_stack.push(prefixes.clone());
        dbg!(&attributes);

        let namespace = self.lookup_prefix(
            &prefix.clone().map(|prefix| prefix.as_str().to_owned())
        )
            .ok_or(Error::MissingNamespace)?
            .to_owned();
        let el = Element::new(
            name.as_string(),
            namespace,
            Some(prefix.map(|prefix| prefix.as_str().to_owned())),
            prefixes,
            attributes,
            vec![]
        );
        self.stack.push(el);

        Ok(())
    }

    fn process_end_tag(&mut self) -> Result<(), Error> {
        if let Some(el) = self.pop() {
            if self.depth() > 0 {
                let top = self.stack.len() - 1;
                self.stack[top].append_child(el);
            } else {
                self.root = Some(el);
            }
        }

        Ok(())
    }

    fn process_text(&mut self, text: String) {
        if self.depth() > 0 {
            let top = self.stack.len() - 1;
            self.stack[top].append_text_node(text);
        }
    }

    /// Process a Event that you got out of a Eventizer
    pub fn process_event(&mut self, event: Event) -> Result<(), Error> {
        dbg!(&event);
        match event {
            Event::XMLDeclaration(_, _) => {},

            Event::StartElement(_, name, attrs) =>
                self.process_start_tag(name, attrs)?,

            Event::EndElement(_) =>
                self.process_end_tag()?,

            Event::Text(_, text) =>
                self.process_text(text.as_string()),
        }

        Ok(())
    }
}
