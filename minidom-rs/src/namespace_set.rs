use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq)]
pub struct NamespaceSet {
    parent: RefCell<Option<Rc<NamespaceSet>>>,
    namespaces: BTreeMap<Option<String>, String>,
}

impl Default for NamespaceSet {
    fn default() -> Self {
        NamespaceSet {
            parent: RefCell::new(None),
            namespaces: BTreeMap::new(),
        }
    }
}

impl fmt::Debug for NamespaceSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NamespaceSet(")?;
        for (prefix, namespace) in &self.namespaces {
            write!(
                f,
                "xmlns{}={:?}, ",
                match prefix {
                    None => String::new(),
                    Some(prefix) => format!(":{}", prefix),
                },
                namespace
            )?;
        }
        write!(f, "parent: {:?})", *self.parent.borrow())
    }
}

impl NamespaceSet {
    pub fn declared_ns(&self) -> &BTreeMap<Option<String>, String> {
        &self.namespaces
    }

    pub fn get(&self, prefix: &Option<String>) -> Option<String> {
        match self.namespaces.get(prefix) {
            Some(ns) => Some(ns.clone()),
            None => match *self.parent.borrow() {
                None => None,
                Some(ref parent) => parent.get(prefix),
            },
        }
    }

    pub fn has<NS: AsRef<str>>(&self, prefix: &Option<String>, wanted_ns: NS) -> bool {
        match self.namespaces.get(prefix) {
            Some(ns) => ns == wanted_ns.as_ref(),
            None => match *self.parent.borrow() {
                None => false,
                Some(ref parent) => parent.has(prefix, wanted_ns),
            },
        }
    }

    pub fn set_parent(&self, parent: Rc<NamespaceSet>) {
        let mut parent_ns = self.parent.borrow_mut();
        let new_set = parent;
        *parent_ns = Some(new_set);
    }
}

impl From<BTreeMap<Option<String>, String>> for NamespaceSet {
    fn from(namespaces: BTreeMap<Option<String>, String>) -> Self {
        NamespaceSet {
            parent: RefCell::new(None),
            namespaces,
        }
    }
}

impl From<Option<String>> for NamespaceSet {
    fn from(namespace: Option<String>) -> Self {
        match namespace {
            None => Self::default(),
            Some(namespace) => Self::from(namespace),
        }
    }
}

impl From<String> for NamespaceSet {
    fn from(namespace: String) -> Self {
        let mut namespaces = BTreeMap::new();
        namespaces.insert(None, namespace);

        NamespaceSet {
            parent: RefCell::new(None),
            namespaces,
        }
    }
}

impl From<(Option<String>, String)> for NamespaceSet {
    fn from(prefix_namespace: (Option<String>, String)) -> Self {
        let (prefix, namespace) = prefix_namespace;
        let mut namespaces = BTreeMap::new();
        namespaces.insert(prefix, namespace);

        NamespaceSet {
            parent: RefCell::new(None),
            namespaces,
        }
    }
}

impl From<(String, String)> for NamespaceSet {
    fn from(prefix_namespace: (String, String)) -> Self {
        let (prefix, namespace) = prefix_namespace;
        Self::from((Some(prefix), namespace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_has() {
        let namespaces = NamespaceSet::from("foo".to_owned());
        assert_eq!(namespaces.get(&None), Some("foo".to_owned()));
        assert!(namespaces.has(&None, "foo"));
    }

    #[test]
    fn get_has_prefixed() {
        let namespaces = NamespaceSet::from(("x".to_owned(), "bar".to_owned()));
        assert_eq!(
            namespaces.get(&Some("x".to_owned())),
            Some("bar".to_owned())
        );
        assert!(namespaces.has(&Some("x".to_owned()), "bar"));
    }

    #[test]
    fn get_has_recursive() {
        let mut parent = NamespaceSet::from("foo".to_owned());
        for _ in 0..1000 {
            let namespaces = NamespaceSet::default();
            namespaces.set_parent(Rc::new(parent));
            assert_eq!(namespaces.get(&None), Some("foo".to_owned()));
            assert!(namespaces.has(&None, "foo"));
            parent = namespaces;
        }
    }

    #[test]
    fn get_has_prefixed_recursive() {
        let mut parent = NamespaceSet::from(("x".to_owned(), "bar".to_owned()));
        for _ in 0..1000 {
            let namespaces = NamespaceSet::default();
            namespaces.set_parent(Rc::new(parent));
            assert_eq!(
                namespaces.get(&Some("x".to_owned())),
                Some("bar".to_owned())
            );
            assert!(namespaces.has(&Some("x".to_owned()), "bar"));
            parent = namespaces;
        }
    }

    #[test]
    fn debug_looks_correct() {
        let parent = NamespaceSet::from("http://www.w3.org/2000/svg".to_owned());
        let namespaces = NamespaceSet::from((
            "xhtml".to_owned(),
            "http://www.w3.org/1999/xhtml".to_owned(),
        ));
        namespaces.set_parent(Rc::new(parent));
        assert_eq!(format!("{:?}", namespaces), "NamespaceSet(xmlns:xhtml=\"http://www.w3.org/1999/xhtml\", parent: Some(NamespaceSet(xmlns=\"http://www.w3.org/2000/svg\", parent: None)))");
    }
}