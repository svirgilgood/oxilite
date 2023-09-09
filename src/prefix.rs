use std::cmp::Ordering;
use std::collections::HashMap;
// use std::collections::hash_map::Iter;
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
  sorted: bool
}

impl Prefix {
  pub fn new() -> Prefix {
    return Prefix { map: HashMap::new(), list: Vec::new(), sorted: false}
  }
  pub fn add(&mut self, namespace: &[u8], prefix: &[u8]) {
    if self.map.contains_key(namespace) {
      return
    }
    self.map.insert(namespace.clone().into(), prefix.clone().into());

    self.list.push(Box::new(namespace.clone().to_vec()));
  }

  // commenting out this method as we don't need an iterator... yet
  // pub fn iter(&self) -> Iter<Box<[u8]>, Box<[u8]>> {
  //   return self.map.iter();
  // }

  pub fn sort(&mut self) {
    self.list.sort_by(|a, b| {
      if b.len() > a.len() { return Ordering::Greater };
      if b.len() < a.len() { return Ordering::Less };
      return Ordering::Equal }
    );
  }

  pub fn get<'a>(&self, namespace: &'a [u8]) -> Option<Box<[u8]>> {
    let prefix = self.map.get(namespace)?;
    return Some(prefix.clone());
  }

  pub fn shorten_uri(&mut self, uri: &str) -> String {
    if !self.sorted { self.sort()}

    let uri_bytes = transform_to_bytes(uri);

    let list = &self.list;
    for namespace in list {
      let is_matched = match_namespace(uri_bytes, &namespace);
      if let Some((namespace, index)) = is_matched {
        if let Some(prefix) =  self.get(namespace) {
          let local = &uri_bytes[index..];
          let local_name = std::str::from_utf8(local).unwrap();
          let prefix_str = std::str::from_utf8(&prefix).unwrap();
          return format!("{prefix_str}:{local_name}");
        }
      }
    }
    return uri.to_string();
  }
}

fn transform_to_bytes<'a>(uri: &'a str) -> &'a [u8]{
  // let mut uri_bytes = Box::<Vec<u8>>::new(uri.as_bytes().to_owned());
  let mut uri_bytes = uri.as_bytes();
  if uri_bytes[0] == 34 {
    uri_bytes = &uri_bytes[1..];
  }
  if uri_bytes[uri_bytes.len()-1] == 34 {
    uri_bytes = &uri_bytes[..uri_bytes.len()-1];
  }
  uri_bytes
}

fn match_namespace<'b>(uri: &[u8], namespace: &'b Vec<u8>) -> Option<(&'b Vec<u8>, usize)> {
  let mut i  = 0;
  for char in namespace.into_iter() {
    if &uri[i] != char {
      return None
    }
    i += 1;
  }
  return Some((namespace, i))
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

}