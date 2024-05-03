use std::cmp::Ordering;
use std::collections::HashMap;
// use std::collections::hash_map::Iter;
use oxigraph::model::vocab::{rdf, xsd};
use oxigraph::model::{GraphName, Literal, NamedNode, Quad};
use oxigraph::store::{StorageError, Store};
use regex::bytes::Regex;

/// Function for Finding Prefixes with Regex
pub fn find_prefixes(file_contents: &Vec<u8>, ns_dict: &mut Prefix) {
    let re: Regex = Regex::new(r"(?i)prefix\s+([\w\d\-_]+):\s+<([a-zA-Z0-9\/:\-.#_]+)>").unwrap();
    for (_, [prefix, namespace]) in re.captures_iter(file_contents).map(|c| c.extract()) {
        ns_dict.add(namespace, prefix);
    }
}

pub struct Prefix {
    map: HashMap<Box<[u8]>, Box<[u8]>>,
    pub list: Vec<Box<Vec<u8>>>,
    sorted: bool,
}

impl Prefix {
    pub fn new() -> Prefix {
        return Prefix {
            map: HashMap::new(),
            list: Vec::new(),
            sorted: false,
        };
    }
    pub fn add(&mut self, namespace: &[u8], prefix: &[u8]) {
        if self.map.contains_key(namespace) {
            return;
        }
        self.map
            .insert(namespace.clone().into(), prefix.clone().into());

        self.list.push(Box::new(namespace.clone().to_vec()));
    }

    // commenting out this method as we don't need an iterator... yet
    // pub fn iter(&self) -> Iter<Box<[u8]>, Box<[u8]>> {
    //   return self.map.iter();
    // }

    pub fn sort(&mut self) {
        self.list.sort_by(|a, b| {
            if b.len() > a.len() {
                return Ordering::Greater;
            };
            if b.len() < a.len() {
                return Ordering::Less;
            };
            return Ordering::Equal;
        });
    }

    pub fn get<'a>(&self, namespace: &'a [u8]) -> Option<Box<[u8]>> {
        let prefix = self.map.get(namespace)?;
        return Some(prefix.clone());
    }

    /// I am not sure how much I like this implementation of save_to_store
    /// It works, but I am not sure the model is correct, or that it should be a method on the ns_dict struct
    pub fn save_to_store(&self, store: &mut Store) -> Result<(), StorageError> {
        let sh_prefix = NamedNode::new("http://www.w3.org/ns/shacl#prefix").unwrap();
        let sh_namespace = NamedNode::new("http://www.w3.org/ns/shacl#namespace").unwrap();
        let sh_declaration =
            NamedNode::new("http://www.w3.org/ns/shacl#PrefixDeclaration").unwrap();
        for (ns, pfx) in &self.map {
            let prefix = std::str::from_utf8(&pfx).unwrap();
            let namespace = std::str::from_utf8(&ns).unwrap();

            let prefix_declaration =
                NamedNode::new(format!("https://sparqlite.github.io/_{prefix}")).unwrap();
            let type_quad = Quad::new(
                prefix_declaration.clone(),
                rdf::TYPE,
                sh_declaration.clone(),
                GraphName::DefaultGraph,
            );
            let prefix_quad = Quad::new(
                prefix_declaration.clone(),
                sh_prefix.clone(),
                Literal::new_typed_literal(prefix, xsd::STRING),
                GraphName::DefaultGraph,
            );
            let namespace_quad = Quad::new(
                prefix_declaration.clone(),
                sh_namespace.clone(),
                Literal::new_typed_literal(namespace, xsd::STRING),
                GraphName::DefaultGraph,
            );
            store.insert(&type_quad)?;
            store.insert(&prefix_quad)?;
            store.insert(&namespace_quad)?;
        }
        Ok(())
    }

    pub fn shorten_uri(&mut self, uri: &str) -> String {
        if !self.sorted {
            self.sort()
        }

        let uri_bytes = transform_to_bytes(uri);

        let list = &self.list;
        for namespace in list {
            let is_matched = match_namespace(uri_bytes, &namespace);
            if let Some((namespace, index)) = is_matched {
                if let Some(prefix) = self.get(namespace) {
                    let local = &uri_bytes[index..];
                    let local_name = std::str::from_utf8(local).unwrap();
                    let prefix_str = std::str::from_utf8(&prefix).unwrap();
                    return format!("{prefix_str}:{local_name}");
                }
            }
        }
        return uri.to_string();
    }

    pub fn format_for_query(&self) -> String {
        let mut prefixes = String::new();
        for namespace in self.list.iter() {
            if let Some(prefix) = self.get(&namespace) {
                let pref_str = String::from_utf8(prefix.to_vec()).unwrap();
                let ns_string = String::from_utf8(namespace.to_vec()).unwrap();
                let line = format!("PREFIX {pref_str}: <{ns_string}>");
                prefixes = format!("{prefixes}\n{line}");
            }
        }
        prefixes = format!("{prefixes}\n");
        prefixes.to_owned()
    }
}

fn transform_to_bytes<'a>(uri: &'a str) -> &'a [u8] {
    // let mut uri_bytes = Box::<Vec<u8>>::new(uri.as_bytes().to_owned());
    let mut uri_bytes = uri.as_bytes();
    if uri_bytes[0] == 34 {
        uri_bytes = &uri_bytes[1..];
    }
    if uri_bytes[uri_bytes.len() - 1] == 34 {
        uri_bytes = &uri_bytes[..uri_bytes.len() - 1];
    }
    uri_bytes
}

fn match_namespace<'b>(uri: &[u8], namespace: &'b Vec<u8>) -> Option<(&'b Vec<u8>, usize)> {
    let mut i = 0;
    for char in namespace.into_iter() {
        if &uri[i] != char {
            return None;
        }
        i += 1;
    }
    return Some((namespace, i));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_shorten_uri() {
        let mut ns_dict = Prefix::new();
        let namespace = "https://example.com/".as_bytes();
        let prefix = "ex".as_bytes();

        ns_dict.add(&namespace, &prefix);

        let uri = "https://example.com/_test_example";
        let res = ns_dict.shorten_uri(uri);
        assert!(res == "ex:_test_example");
    }
    #[test]
    fn should_shorten_escaped_quotes() {
        let mut ns_dict = Prefix::new();
        let namespace = "http://www.w3.org/2000/01/rdf-schema#".as_bytes();
        let prefix = "rdf".as_bytes();

        ns_dict.add(&namespace, &prefix);

        let uri = "\"http://www.w3.org/2000/01/rdf-schema#comment\"";
        let res = ns_dict.shorten_uri(uri);
        assert_eq!(res, "rdf:comment");
    }

    #[test]
    fn should_return_formatted_prefixes() {
        let mut ns_dict = Prefix::new();

        let rdf_ns = "http://www.w3.org/2000/01/rdf-schema#".as_bytes();
        let rdf_pref = "rdf".as_bytes();
        ns_dict.add(&rdf_ns, &rdf_pref);

        let ex_ns = "https://example.com/".as_bytes();
        let ex_pref = "ex".as_bytes();
        ns_dict.add(&ex_ns, &ex_pref);
        ns_dict.sort();

        let expected_result = "\nPREFIX rdf: <http://www.w3.org/2000/01/rdf-schema#>\nPREFIX ex: <https://example.com/>\n";
        assert_eq!(expected_result, ns_dict.format_for_query());
    }
}
