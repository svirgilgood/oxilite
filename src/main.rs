use clap::{ArgAction, Parser};
use oxigraph::{
    model::{ Term, QuadRef },
    io,
    //sparql::QueryResults,
    sparql::QuerySolution,
    sparql::QueryOptions,
    store::{StorageError, LoaderError },
    store,
    sparql::results::{QueryResultsFormat, QueryResultsSerializer },
};
use oxigraph::io::JsonLdProfile;
use oxigraph::sparql::EvaluationError;
use oxigraph::sparql;
use std::io::Read;
use std::env;
use std::path::Path;



use oxigraph::io::{RdfFormat, RdfSerializer};
use oxrdfio::RdfParser;

use comfy_table::{Table, ContentArrangement};
//use serde_derive::Deserialize;
//use serde_json::Map;
use std::{fs, str, io::Cursor, path::PathBuf};

mod prefix;
use crate::prefix::{find_prefixes, Prefix};
mod repl;
use crate::repl::readlinefn;
mod remote_store;
use crate::remote_store::{ SparqlJson, Query, QueryResults, RemoteStore };

impl Query for store::Store {
    fn explain_query(&mut self, query: &str, ns_dict: &Prefix) -> QueryResults {
     let (results, _explanation) = self.explain_query_opt(query, QueryOptions::default(), true).unwrap();
        match results.unwrap() {
            sparql::QueryResults::Solutions(solutions) => {
                let mut writer: Vec<_> = Vec::new();
                //let res = solutions. .write(&mut writer, QueryResultsFormat::Json);
                let json_serializer = QueryResultsSerializer::from_format(QueryResultsFormat::Json);
                let mut serializer = json_serializer.serialize_solutions_to_writer(&mut writer, solutions.variables().to_vec().clone()).unwrap();
                for solution in solutions {
                    serializer.serialize(&solution.unwrap()).unwrap();
                }
                serializer.finish().unwrap();
                let object: SparqlJson = serde_json::from_slice(&writer).expect("Error in Parsing Json");
                QueryResults::Solutions(object.to_owned())
            },
            sparql::QueryResults::Boolean(result) => {
                QueryResults::Boolean(result)
            },
            sparql::QueryResults::Graph(triples) => {
                let mut tserializer = RdfSerializer::from_format(RdfFormat::Turtle); //.for_writer(Vec::new());
                for (prefix, namespace) in ns_dict.fetch_namespace_prefix() {
                    tserializer = tserializer.with_prefix(std::str::from_utf8(&prefix).unwrap(), std::str::from_utf8(&namespace).unwrap()).unwrap();
                }

                let mut serializer = tserializer.for_writer(Vec::new());


                for triple in triples {
                    serializer.serialize_triple(triple.unwrap().as_ref()).unwrap();
                }

                //let final_ser = serializer.finish().unwrap();
                let res = match str::from_utf8(&serializer.finish().unwrap()) {
                    Ok(res) => res.to_owned(), //println!("{}", res),
                    _ => "Error in parsing rdf string".to_owned()
                };
                QueryResults::Graph(res.to_string())
            }
        }
    }
    fn query(&self, query: &str) -> Result<sparql::QueryResults, EvaluationError> {
        <store::Store>::query(self, query)
    }
    fn len(&self) -> usize {
        <store::Store>::len(self).unwrap_or(11111)
    }
    fn load_from_reader(&mut self, parser: impl Into<io::RdfParser>, reader: impl Read) -> Result<(), LoaderError> {
        <store::Store>::load_from_reader(self, parser, reader)
    }
    fn insert<'a>(&mut self, quad: impl Into<QuadRef<'a>>) -> Result<bool, StorageError>  {
        <store::Store>::insert(self, quad)
    }
    fn optimize(&mut self) -> Result<(), StorageError> {
        <store::Store>::optimize(self)
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None )]
struct Args {
    /// Name of the directory or file for trig/nq files, argument can be repeated
    #[arg(short, long)]
    data: Vec<String>,

    /// Name of the file or string for loading the query. Using `\n` as the separator in
    /// `GROUP_CONCAT()` will result in new lines in the results table.
    #[arg(short, long)]
    query: Option<String>,

    /// Print the query before executing
    #[arg(long, action=ArgAction::SetTrue)]
    print_query: bool,

    /// Use or create a saved database. By specifying the database these will be stored
    /// or they will re-use the exiting database
    #[arg(long)]
    db: Option<String>,

    /// Toggle prefix injection. For inline queries the default
    /// is to inject the prefixes into the query, but for file based queries,
    /// the default is to not inject the prefixes
    #[arg(long, action=ArgAction::SetFalse)]
    toggle_prefix: bool,

    /// Add credentials for In the format Username:Password. If username but no password,
    /// the user will be prompted for a password And the password will not be stored
    #[arg(short, long)]
    credentials: Option<String>,

    /// Add prefixes in the form of a turtle file, including multi-line separation.
    #[arg(short, long)]
    prefixes: Option<String>,

    /// Update the config with the data passed through command line flags
    #[arg(long, action=ArgAction::SetTrue)]
    update_config: bool,

    /// Run optimization on an ondisk database, this option is ignored if the --db flag is not
    /// selected
    #[arg(long, action=ArgAction::SetTrue)]
    optimize_database: bool,

    // Needed Options:
    // --inline-prefix: Vec<String> 
    // --prefix: Option<String>
    // --credentials: Option<String> In the format Username:Password. If username but no password,
    //                              ask for password.
    // --update-config: bool Use the input values to update the config 
}

fn update_store(store: &mut QStore, path: PathBuf, ns_dict: &mut Prefix) -> Option<()> {
    let ext = path.extension()?;
    let name = path.file_name()?.to_ascii_lowercase();

    if ext.is_empty() {
        return None;
    }
    let file_format = match ext.to_str().unwrap() {
        "ttl" => RdfFormat::Turtle,
        "trig" => RdfFormat::TriG,
        "n3" => RdfFormat::N3,
        "rdf" => RdfFormat::RdfXml,
        //"json" => RdfFormat::JsonLd(JsonLdProfile::Expanded),
        _ => RdfFormat::Turtle
    };
    let file = fs::read(path);

    if file.is_err() {
        return None;
    }

    let file_contents = file.unwrap();
    find_prefixes(&file_contents, ns_dict);
    let res = store.load_from_reader(RdfParser::from_format(file_format), Cursor::new(&file_contents));
    if res.is_err() {
        println!("Error: {:?}", res);
        println!("Error saving {:?} to store", name);
        return None;
    }

    Some(())
}

fn print_select(solutions: SparqlJson, ns_dict: &mut Prefix) {

//    let mut writer: Vec<_> = Vec::new();
//    //let res = solutions. .write(&mut writer, QueryResultsFormat::Json);
//    let json_serializer = QueryResultsSerializer::from_format(QueryResultsFormat::Json);
//    let mut serializer = json_serializer.serialize_solutions_to_writer(&mut writer, solutions.variables().to_vec()).unwrap();
//
//    for solution in solutions {
//        serializer.serialize(&solution.unwrap()).unwrap();
//    }
//
//    serializer.finish().unwrap();
//
//    let object: SparqlJson = serde_json::from_slice(&writer).expect("Error in Parsing Json");

    // This is where we need to pass the objects
    let vars = solutions.head;

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    let headings: Vec<String> =  vars.vars
            .clone()
            .into_iter()
            .map(|x| x.to_string())
           .collect();
    // the following loop should really be placed in its own function
    // perhaps a module and re-write the pretty printing of the table
    table.set_header(headings);
    for result in solutions.results.bindings {
        let mut print_res  = Vec::new();
        for var in &vars.vars {
            if let Some(serde_json::Value::Object(var_map)) = &result.get(&var.to_string()).or(None)
            {
                let rdf_type = &var_map["type"];
                let let_return_value = match rdf_type.as_str() {
                    Some("uri") => {
                        let res = ns_dict.shorten_uri(&var_map["value"].to_string());
                        res
                    }
                    Some("literal") => str::replace(&var_map["value"].to_string(), "\\n", "\n"),
                    Some("bnode") => var_map["value"].to_string(),
                    Some("triple") => format!(
                        "{}\t{}\t{}",
                        var_map["subject"], var_map["predicate"], var_map["object"]
                    ),
                    _ => continue,
                };
                print_res.push(let_return_value);
            } else {
                // This happens when there is no particular result for the variable, we need to set a place holder
                // This allows the cell to be empty
                print_res.push("".to_string())
            }
        }
        table.add_row(print_res);
    }
    //table.printstd();
    println!("{table}");
    let row_numbers = table.row_count();
    println!("Total: {}", row_numbers);
}


/// We are going to keep this function in case we want to add formatting here
fn print_graph(triples: &str, ns_dict: &Prefix)  {
    println!("{}", triples);

//    let mut tserializer = RdfSerializer::from_format(RdfFormat::Turtle); //.for_writer(Vec::new());
//    for (prefix, namespace) in ns_dict.fetch_namespace_prefix() {
//        tserializer = tserializer.with_prefix(std::str::from_utf8(&prefix).unwrap(), std::str::from_utf8(&namespace).unwrap()).unwrap();
//    }
//
//    let mut serializer = tserializer.for_writer(Vec::new());
//
//
//    for triple in triples {
//        serializer.serialize_triple(triple.unwrap().as_ref()).unwrap();
//    }
//
//    //let final_ser = serializer.finish().unwrap();
//    match str::from_utf8(&serializer.finish().unwrap()) {
//        Ok(res) => println!("{}", res),
//        _ => println!("Error in parsing string")
//    }
}

fn print_query(
    store: &mut QStore,
    query: &str,
    ns_dict: &mut Prefix,
    print: bool,
    is_prefix_injected: bool,
) {
    let prefix_string = ns_dict.format_for_query();
    let formatted_query = if is_prefix_injected {
        format!("{prefix_string}\n\n{query}")
    } else {
        query.to_string()
    };

    if print {
        println!("{}\n\n", formatted_query);
    }

    //let (results, _explanation) = store.explain_query_opt(&formatted_query, QueryOptions::default(), true).unwrap();
    let results = store.explain_query(&formatted_query, ns_dict);
    match results {
        QueryResults::Solutions(solutions) => {
            print_select(solutions, ns_dict);
        },
        QueryResults::Boolean(result) => {
            println!("{:?}", result);
        },
        QueryResults::Graph(triples) => {
            print_graph(&triples, ns_dict);
        }
    }

}

///
/// Takes a Prefix dictionary and a store, and updates the dictionary based on the
/// existing prefixes in the database
/// The query that creates these is the following SPARQL
///
/// ```
/// SELECT ?prefix ?namespace
/// WHERE {
///    ?declaration
///        a sh:PrefixDeclaration ;
///        sh:prefix ?prefix ;
///        sh:namespace ?namespace ;
///    .
/// }
///````
/// When querying the config file a `FROM` clause will be added
fn get_namespaces(ns_dict: &mut Prefix, store: &QStore, named_graph: Option<&str>) {

    let prefix = "PREFIX sh: <http://www.w3.org/ns/shacl#>

SELECT ?prefix ?namespace
        ";
    let graph_pattern = "
WHERE {
    ?declaration
        a sh:PrefixDeclaration ;
        sh:prefix ?prefix ;
        sh:namespace ?namespace ;
    .
}
        ";
    let query = match named_graph { 
        Some(graph_uri) => format!("{}\nFROM <{}>\n{}", prefix, graph_uri, graph_pattern),
        None => format!("{}\n{}", prefix, graph_pattern)
    };


    // This lambda function is about simplifying the turning of a Solution Term into a String
    // to simplify the creation of the dictionary entry
    let term_getter = |solution: &QuerySolution, variable: &str| -> String {
        let term = solution.get(variable).unwrap();
        let value = match term {
            Term::Literal(v) => {
                let (value, _, _) = v.clone().destruct();
                value
            }
            _ => term.to_string(),
        };
        value
    };

    if let sparql::QueryResults::Solutions(solutions) = store.query(&query).expect("Error in query Results")
    {
        for solution in solutions.filter_map(|x| x.ok()) {
            let namespace = term_getter(&solution, "namespace");
            let prefix = term_getter(&solution, "prefix");
            ns_dict.add(
                namespace.to_string().as_bytes(),
                prefix.to_string().as_bytes(),
            );
        }
    }
}

fn create_remote_store(url: &str, args: &Args) -> RemoteStore {
    match &args.credentials {
        Some(creds) => {
            let credentials: Vec<&str> = creds.split(":").collect();
            if credentials.len() == 0 {
                return RemoteStore::new(url, None, None)
            }
            let user = Some(credentials[0].to_string());
            if credentials.len() == 1 {
                return RemoteStore::new(url, user.to_owned(), None)
            }
            let password = Some(credentials[1].to_string());
            RemoteStore::new(url, user.to_owned(), password.to_owned())

        },
        None => RemoteStore::new(url, None, None)
    }
}

enum QStore {
    RemoteStore(RemoteStore),
    Store(store::Store),
}

impl Query for QStore {
    fn explain_query(&mut self, query: &str, ns_dict: &Prefix) -> QueryResults {
        match self {
            QStore::RemoteStore(store) => store.explain_query(query, ns_dict),
            QStore::Store(store) => store.explain_query(query, ns_dict),
        }
        //self.explain_query(query, ns_dict)
    }
    fn query(&self, query: &str) -> Result<sparql::QueryResults, EvaluationError> {
        match self {
            QStore::RemoteStore(store) => store.query(query),
            QStore::Store(store) => store.query(query)
        }
        //self.query(query)
    }
    fn len(&self) -> usize {
        match self {
            QStore::RemoteStore(store) => store.len(),
            QStore::Store(store) => store.len().unwrap_or(1),
        }
        //self.len()
    }
    fn load_from_reader(&mut self, parser: impl Into<io::RdfParser>, reader: impl Read) -> Result<(), LoaderError> {
        match self {
            QStore::RemoteStore(store) => store.load_from_reader(parser, reader),
            QStore::Store(store) => store.load_from_reader(parser, reader)
        }
        //self.load_from_reader(parser, reader)
    }
    fn insert<'a>(&mut self, quad: impl Into<QuadRef<'a>>) -> Result<bool, StorageError> {
        //self.insert(quad)
        match self {
            QStore::RemoteStore(store) => store.insert(quad),
            QStore::Store(store) => store.insert(quad)
        }
    }
    fn optimize(&mut self) -> Result<(), StorageError> {
        match self {
            QStore::RemoteStore(store) => store.optimize(),
            QStore::Store(store) => store.optimize()
        }
    }
}

fn main() {
    let args = Args::parse();

    // 
    let mut is_remote = false;

    // Store::open is used for an on disk database, it will work even if the the
    // store doesn't exist, Oxigraph will create it
    let mut store: QStore = match args.db {
        Some(str) => {
            let path = std::path::Path::new(&str);
            QStore::Store(store::Store::open(path).unwrap())
        },
        // Store::new() will create an in memory store that will drop after the script finishes
        _ => { 
            let remote_store: Vec<String> = args.data.iter().filter(|x| x.starts_with("http")).cloned().collect();
            if remote_store.len() == 1 {
                is_remote = true;
                QStore::RemoteStore(create_remote_store(&remote_store[0], &args))
            } else {
                QStore::Store(store::Store::new().unwrap())
            }

        }
    };

    let mut ns_dict: Prefix = Prefix::new();
    let mut config = QStore::Store(store::Store::new().unwrap());
    update_store(&mut config, Path::new(&env::var("HOME").unwrap()).join(".config").join("sparqlite").join("config.trig").to_path_buf(), &mut ns_dict);

    if let Some(prefixes) = args.prefixes {
        println!("\n***Found prefixes****\n");
        let prefix_vec = prefixes.as_bytes().to_owned();
        find_prefixes(&prefix_vec, &mut ns_dict);
        let graph_name = if is_remote { 
            Some(args.data[0].as_str())
        } else { 
            None
        };
        ns_dict.save_to_store(&mut config, graph_name) ;
        let prefix_string = ns_dict.format_for_query();
        println!("Prefix String: {}", prefix_string);

    }

    if !is_remote {
        for data in &args.data {
            let metadata = fs::metadata(data);

            match metadata {
                Ok(file_type) => {
                    if file_type.is_dir() {
                        let paths = fs::read_dir(&data).unwrap();
                        for path in paths {
                            if path.is_err() {
                                println!("Path contains error: {:?}", path);
                                continue;
                            };
                            update_store(&mut store, path.unwrap().path(), &mut ns_dict);
                        }
                        if let Err(e) = ns_dict.save_to_store(&mut store, None) {
                            println!("{:?}", e);
                            panic!("Error in Save to Store");
                        };
                    } else {
                        update_store(&mut store, PathBuf::from(data), &mut ns_dict);
                        if let Err(e) = ns_dict.save_to_store(&mut store, None) {
                            println!("{:?}", e);
                            panic!("Error in Save to Store");
                        };
                    }
                }
                Err(e) => println!("File does not exist: {}\n with error {}", data, e),
            }
        }
    } else {
        get_namespaces(&mut ns_dict, &config, Some(&args.data[0]));
        //println!("Prefix String After get_namespaces: {}", ns_dict.format_for_query() )
    };

    // if there is a directory supplied, the namespaces are supplied in the files
    // if there is no directory supplied, it needs to be grabbed from the prefixes stored
    // in the databases
    if args.data.len() == 0 {
        //if &args.data == &None {
        get_namespaces(&mut ns_dict, &store, None)
    };

    let length = store.len();
    if length == 0 {
    //if length.is_err() || length.unwrap() == 0 {
        println!("Error in loading datasets");
        return;
    }

    let query = match args.query {
        Some(str) => str,
        None => {
            let q = readlinefn(&ns_dict);
            match q {
                Some(str) => str,
                None => panic!("Error in readline"),
            }
        }
    };


    if args.optimize_database == true {
        println!("Optimization called");
        store.optimize().expect("problems with optimization");
    }

    if std::path::Path::new(&query).exists() {
        let read_file = fs::read_to_string(&query);
        if read_file.is_err() {
            println!("There is an error in reading the query file");
            return;
        }
        print_query(
            &mut store,
            &read_file.unwrap(),
            &mut ns_dict,
            args.print_query,
            !args.toggle_prefix,
        );

    }
    else {
    // println!("query: {query}");
        print_query(
            &mut store,
            &query,
            &mut ns_dict,
            args.print_query,
            args.toggle_prefix,
        );
    }

    if args.update_config {


    }
}


