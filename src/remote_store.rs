use oxigraph::model::QuadRef;
use oxigraph::sparql::QueryEvaluationError;
use oxigraph::store::{LoaderError, StorageError};
use oxigraph::{io, sparql};
use reqwest;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde_derive::Deserialize;
use serde_json::Map;
use std::error::Error;
use std::io::Read;
use tokio;

use crate::prefix::Prefix;

#[derive(Deserialize, Clone)]
pub struct SparqlJson {
    pub head: HeadJson,
    pub results: ResultJson,
}

#[derive(Deserialize, Clone)]
pub struct HeadJson {
    pub vars: Vec<Box<str>>,
}

#[derive(Deserialize, Clone)]
pub struct ResultJson {
    pub bindings: Vec<Map<String, serde_json::Value>>,
}

/// The RemoteStore needs to have the same traits that are impl for the
/// oxigraph store, however, it also needs to allow different configuration
/// options to be loaded locally.
///
pub struct RemoteStore {
    pub url: String,
    pub username: Option<String>,
    password: Option<String>,
}

impl RemoteStore {
    pub fn new(url: &str, username: Option<String>, password: Option<String>) -> RemoteStore {
        RemoteStore {
            url: url.to_string(),
            username: username,
            password: password,
        }
    }

    pub fn set_user(&mut self, user: &str, password: Option<&str>) {
        self.username = Some(user.to_string());
        if let Some(pswd) = password {
            self.password = Some(pswd.to_string());
        };
    }
}

pub enum QueryResults {
    Solutions(SparqlJson),
    Boolean(bool),
    Graph(String),
}

pub trait Query {
    fn explain_query(&mut self, query: &str, ns_dict: &Prefix) -> QueryResults;
    fn query(&self, query: &str) -> Result<sparql::QueryResults, QueryEvaluationError>;
    fn len(&self) -> usize;
    fn load_from_reader(
        &mut self,
        parser: impl Into<io::RdfParser>,
        reader: impl Read,
    ) -> Result<(), LoaderError>;
    fn insert<'a>(&mut self, quad: impl Into<QuadRef<'a>>) -> Result<(), StorageError>;
    fn optimize(&mut self) -> Result<(), StorageError>;
}

fn fetch_results(
    url: &str,
    query: &str,
    accept: &str,
    user: &Option<String>,
    password: &Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let send_url = url.to_string();
    let send_accept = accept.to_string();
    let send_query = query.to_string();
    let (usr, pswd): (String, Option<String>) = match user {
        Some(usr) => match password {
            Some(pswd) => (usr.clone(), Some(pswd.clone())),
            None => {
                println!("Please enter password");
                let mut pswd = String::new();
                std::io::stdin()
                    .read_line(&mut pswd)
                    .expect("Error with the way that line was read");
                let opswd = if pswd == "" {
                    None
                } else {
                    Some(pswd.trim().to_string())
                };
                (usr.clone(), opswd)
            }
        },
        None => ("".to_string(), None),
    };

    //println!("user: {:?} password {:?}", usr, pswd);

    let handle = runtime.spawn(async {
        let client = reqwest::Client::new();
        //println!("user: {:?} password {:?}", &usr, &pswd);
        let results = client
            .post(send_url)
            .basic_auth(usr, pswd)
            .header(ACCEPT, send_accept)
            .header(CONTENT_TYPE, "application/sparql-query")
            .body(send_query)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        results
    });

    Ok(runtime.block_on(handle).unwrap())
}

impl Query for RemoteStore {
    // .print_query
    // .explain_query_opt() -> from oxigraph
    // .query -> from oxigraph
    // .len -> from oxigraph, just to see if the store is empty
    // .load_from_reader -> used to read RDF files
    // .insert(&quad)
    fn explain_query(&mut self, query: &str, _ns_dict: &Prefix) -> QueryResults {
        let query_obj = spargebra::Query::parse(query, None).unwrap();

        let user: Option<String> = self.username.clone();
        let password: Option<String> = self.password.clone();

        match query_obj {
            spargebra::Query::Select { .. } => {
                let accept = "application/sparql-results+json";
                let result = fetch_results(&self.url, query, accept, &user, &password);
                //if result.await.is_err() {
                //    return Err(EvaluationError::ResultsParsing(Error));
                //}
                let object: SparqlJson = serde_json::from_str(&result.unwrap()).unwrap();
                //Ok(QueryResults::Solutions(object))
                QueryResults::Solutions(object)
            }
            spargebra::Query::Construct { .. } => {
                let accept = "text/turtle";
                let result = fetch_results(&self.url, query, accept, &user, &password);
                //if result.await.is_err() {
                //    return Err(EvaluationError::ResultsParsing(Error));
                //}
                //Ok(QueryResults::Graph(result.await.unwrap().to_string()))
                QueryResults::Graph(result.unwrap().to_string())
            }
            spargebra::Query::Describe { .. } => {
                let accept = "text/turtle";
                let result = fetch_results(&self.url, query, accept, &user, &password);
                //if result.await.is_err() {
                //    return Err(EvaluationError::ResultsParsing(Error));
                //}
                //Ok(QueryResults::Graph(result.await.unwrap().to_string()))
                QueryResults::Graph(result.unwrap().to_string())
            }
            spargebra::Query::Ask { .. } => {
                let accept = "application/sparql-results+json";
                let result = fetch_results(&self.url, query, accept, &user, &password);
                //if result.await.is_err() {
                //    return Err(EvaluationError::ResultsParsing(Error));
                //}
                let res: bool = match result.unwrap().as_str() {
                    "true" => true,
                    _ => false,
                };
                //Ok(QueryResults::Boolean(res))
                QueryResults::Boolean(res)
            }
        }
    }
    fn query(&self, query: &str) -> Result<sparql::QueryResults, QueryEvaluationError> {
        Ok(sparql::QueryResults::Boolean(true))
    }
    fn len(&self) -> usize {
        1
    }
    fn load_from_reader(
        &mut self,
        parser: impl Into<io::RdfParser>,
        reader: impl Read,
    ) -> Result<(), LoaderError> {
        Ok(())
    }
    fn insert<'b>(&mut self, quad: impl Into<QuadRef<'b>>) -> Result<(), StorageError> {
        Ok(())
    }

    fn optimize(&mut self) -> Result<(), StorageError> {
        println!("Remote Store's cannot be optimized at this time");
        Ok(())
    }
}
