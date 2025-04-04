use clap::{ArgAction, Parser};
use oxigraph::{
    model::Term,
    sparql::QueryResults,
    sparql::QuerySolution,
    sparql::QueryOptions,
    store::Store,
    sparql::{ QuerySolutionIter, QueryTripleIter },
    sparql::results::{QueryResultsFormat, QueryResultsSerializer },
};



use oxigraph::io::{RdfFormat, RdfSerializer};
use oxrdfio::RdfParser;

use comfy_table::{Table, ContentArrangement};
use serde_derive::Deserialize;
use serde_json::Map;
use std::{fs, str, io::Cursor, path::PathBuf};

mod prefix;
use crate::prefix::{find_prefixes, Prefix};
mod repl;
use crate::repl::readlinefn;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None )]
struct Args {
    /// Name of the directory or file for trig/nq files, argument can be repeated
    #[arg(short, long)]
    data: Vec<String>,

    /// Name of the file or string for loading the query
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
}

fn update_store(store: &mut Store, path: PathBuf, ns_dict: &mut Prefix) -> Option<()> {
    let ext = path.extension()?;
    let name = path.file_name()?.to_ascii_lowercase();

    if ext.is_empty() {
        return None;
    }
    let file = fs::read(path);

    if file.is_err() {
        return None;
    }

    let file_contents = file.unwrap();
    find_prefixes(&file_contents, ns_dict);
    let res = store.load_from_reader(RdfParser::from_format(RdfFormat::Turtle), Cursor::new(&file_contents));
    if res.is_err() {
        println!("Error: {:?}", res);
        println!("Error saving {:?} to store", name);
        return None;
    }

    Some(())
}

#[derive(Deserialize)]
struct SparqlJson {
    head: HeadJson,
    results: ResultJson,
}

#[derive(Deserialize)]
struct HeadJson {
    vars: Vec<Box<str>>,
}

#[derive(Deserialize)]
struct ResultJson {
    bindings: Vec<Map<String, serde_json::Value>>,
}

fn print_select(solutions: QuerySolutionIter, ns_dict: &mut Prefix) {

    let mut writer: Vec<_> = Vec::new();
    //let res = solutions. .write(&mut writer, QueryResultsFormat::Json);
    let json_serializer = QueryResultsSerializer::from_format(QueryResultsFormat::Json);
    let mut serializer = json_serializer.serialize_solutions_to_writer(&mut writer, solutions.variables().to_vec()).unwrap();

    for solution in solutions {
        serializer.serialize(&solution.unwrap()).unwrap();
    }

    serializer.finish().unwrap();




    let object: SparqlJson = serde_json::from_slice(&writer).expect("Error in Parsing Json");
    let vars = object.head;

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
    for result in object.results.bindings {
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
                    Some("literal") => var_map["value"].to_string(),
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



fn print_graph(triples: QueryTripleIter, ns_dict: &Prefix)  {

    let mut tserializer = RdfSerializer::from_format(RdfFormat::Turtle); //.for_writer(Vec::new());
    for (prefix, namespace) in ns_dict.fetch_namespace_prefix() {
        tserializer = tserializer.with_prefix(std::str::from_utf8(&prefix).unwrap(), std::str::from_utf8(&namespace).unwrap()).unwrap();
    }

    let mut serializer = tserializer.for_writer(Vec::new());


    for triple in triples {
        serializer.serialize_triple(triple.unwrap().as_ref()).unwrap();
    }

    //let final_ser = serializer.finish().unwrap();
    match str::from_utf8(&serializer.finish().unwrap()) {
        Ok(res) => println!("{}", res),
        _ => println!("Error in parsing string")
    }
}

fn print_query(
    store: &Store,
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

    let (results, _explanation) = store.explain_query_opt(&formatted_query, QueryOptions::default(), true).unwrap();
    match results.unwrap() {
        QueryResults::Solutions(solutions) => {
            print_select(solutions, ns_dict);
        },
        QueryResults::Boolean(result) => {
            println!("{:?}", result);
        },
        QueryResults::Graph(triples) => {
            print_graph(triples, ns_dict);
        }
    }

}

///
/// Takes a Prefix dictionary and a store, and updates the dictionary based on the
/// existing prefixes in the database
/// The query that creates these is the following SPARQL
///
/// PREFIX sh: <http://www.w3.org/ns/shacl#>
///
///        store.dump_graph_to_writer(GraphNameRef::DefaultGraph, RdfFormat::NTriples, &mut buffer)?;
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
fn get_namespaces(ns_dict: &mut Prefix, store: &Store) {
    let query = "
PREFIX sh: <http://www.w3.org/ns/shacl#>

SELECT ?prefix ?namespace
WHERE {
    ?declaration
        a sh:PrefixDeclaration ;
        sh:prefix ?prefix ;
        sh:namespace ?namespace ;
    .
}
        ";
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

    if let QueryResults::Solutions(solutions) = store.query(query).expect("Error in query Results")
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

fn main() {
    let args = Args::parse();

    // Store::open is used for an on disk database, it will work even if the the
    // store doesn't exist, Oxigraph will create it
    let mut store = match args.db {
        Some(str) => {
            let path = std::path::Path::new(&str);
            Store::open(path).unwrap()
        }
        // Store::new() will create an in memory store that will drop after the script finishes
        _ => Store::new().unwrap(),
    };

    let mut ns_dict: Prefix = Prefix::new();

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
                    if let Err(e) = ns_dict.save_to_store(&mut store) {
                        println!("{:?}", e);
                        panic!("Error in Save to Store");
                    };
                } else {
                    update_store(&mut store, PathBuf::from(data), &mut ns_dict);
                }
            }
            Err(e) => println!("File does not exist: {}\n with error {}", data, e),
        }
    }

    // if there is a directory supplied, the namespaces are supplied in the files
    // if there is no directory supplied, it needs to be grabbed from the prefixes stored
    // in the databases
    if args.data.len() == 0 {
        //if &args.data == &None {
        get_namespaces(&mut ns_dict, &store)
    };

    let length = store.len();
    if length.is_err() || length.unwrap() == 0 {
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

    if std::path::Path::new(&query).exists() {
        let read_file = fs::read_to_string(&query);
        if read_file.is_err() {
            println!("There is an error in reading the query file");
            return;
        }
        print_query(
            &store,
            &read_file.unwrap(),
            &mut ns_dict,
            args.print_query,
            !args.toggle_prefix,
        );

        return;
    }
    // println!("query: {query}");

    print_query(
        &store,
        &query,
        &mut ns_dict,
        args.print_query,
        args.toggle_prefix,
    );
}


