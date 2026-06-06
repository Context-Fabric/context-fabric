use std::collections::BTreeMap;
use std::sync::Arc;

pub const API_REF_ROWS: &[(&str, (&str, &str, &str))] = &[
    ("CF", ("Fabric", "fabric", "loading")),
    ("TF", ("Fabric", "fabric", "loading")),
    ("F", ("Features", "node", "node-features")),
    ("Feature", ("Features", "node", "node-features")),
    ("Fs", ("Features", "nodestr", "node-features")),
    ("E", ("Features", "edge", "edge-features")),
    ("Edge", ("Features", "edge", "edge-features")),
    ("Es", ("Features", "edgestr", "edge-features")),
    ("C", ("Computed", "computed", "computed-data")),
    ("Computed", ("Computed", "computed", "computed-data")),
    ("Cs", ("Computed", "computedstr", "computed-data")),
    ("N", ("Nodes", "nodes", "navigating-nodes")),
    ("Nodes", ("Nodes", "nodes", "navigating-nodes")),
    ("L", ("Locality", "locality", "locality")),
    ("Locality", ("Locality", "locality", "locality")),
    ("T", ("Text", "text", "text")),
    ("Text", ("Text", "text", "text")),
    ("S", ("Search", "search", "search")),
    ("Search", ("Search", "search", "search")),
];

#[derive(Debug, Clone)]
#[allow(non_snake_case)]
pub struct Api<T> {
    pub CF: Arc<T>,
    pub TF: Arc<T>,
    pub ignored: Vec<String>,
}

impl<T> Api<T> {
    pub fn new(fabric: T) -> Self {
        let fabric = Arc::new(fabric);
        Self {
            CF: Arc::clone(&fabric),
            TF: fabric,
            ignored: Vec::new(),
        }
    }

    pub fn from_arc<I, S>(fabric: Arc<T>, ignored: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut ignored = ignored.into_iter().map(Into::into).collect::<Vec<_>>();
        ignored.sort();
        ignored.dedup();
        Self {
            CF: Arc::clone(&fabric),
            TF: fabric,
            ignored,
        }
    }

    pub fn cf(&self) -> Arc<T> {
        Arc::clone(&self.CF)
    }

    pub fn tf(&self) -> Arc<T> {
        Arc::clone(&self.TF)
    }

    pub fn cf_and_tf_are_same_object(&self) -> bool {
        Arc::ptr_eq(&self.CF, &self.TF)
    }
}

pub fn api_refs() -> BTreeMap<&'static str, (&'static str, &'static str, &'static str)> {
    API_REF_ROWS.iter().copied().collect()
}
