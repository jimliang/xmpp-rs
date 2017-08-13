use std::collections::BTreeMap;
use std::cell::RefCell;
use std::rc::Rc;


#[derive(Clone, Debug, PartialEq, Eq)]
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

impl NamespaceSet {
    pub fn declared_ns(&self) -> &BTreeMap<Option<String>, String> {
        &self.namespaces
    }
    
    pub fn get(&self, prefix: &Option<String>) -> Option<String> {
        match self.namespaces.get(prefix) {
            Some(ns) => Some(ns.clone()),
            None => match *self.parent.borrow() {
                None => None,
                Some(ref parent) => parent.get(prefix)
            },
        }
    }

    pub fn has<NS: AsRef<str>>(&self, prefix: &Option<String>, wanted_ns: NS) -> bool {
        match self.namespaces.get(prefix) {
            Some(ns) =>
                ns == wanted_ns.as_ref(),
            None => match *self.parent.borrow() {
                None =>
                    false,
                Some(ref parent) =>
                    parent.has(prefix, wanted_ns),
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
            namespaces: namespaces,
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
            namespaces: namespaces,
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
            namespaces: namespaces,
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
    use std::rc::Rc;

    #[test]
    fn get_has() {
        let namespaces = NamespaceSet::from("foo".to_owned());
        assert_eq!(namespaces.get(&None), Some("foo".to_owned()));
        assert!(namespaces.has(&None, "foo"));
    }

    #[test]
    fn get_has_prefixed() {
        let namespaces = NamespaceSet::from(("x".to_owned(), "bar".to_owned()));
        assert_eq!(namespaces.get(&Some("x".to_owned())), Some("bar".to_owned()));
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
            assert_eq!(namespaces.get(&Some("x".to_owned())), Some("bar".to_owned()));
            assert!(namespaces.has(&Some("x".to_owned()), "bar"));
            parent = namespaces;
        }
    }

}
