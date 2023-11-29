use oxigraph::{io::DatasetFormat, store::Store, sparql::QueryResultsFormat};
use std::{fs, path::PathBuf, io::Cursor};
use clap::Parser;
use serde_json::Map;
use serde_derive::Deserialize;
use prettytable::{ Table, Row, Cell };


mod prefix;
use crate::prefix::{Prefix, find_prefixes};
mod repl;
use crate::repl::readlinefn;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None )]
struct Args {
    /// name of the directory for trig/nq files
    #[arg(short, long)]
    directory: String,

    /// name of the file or string for loading the query
    #[arg(short, long)]
    query: Option<String>,
}


fn update_store(store: &mut Store, path: PathBuf, ns_dict: &mut Prefix) -> Option<()> {
    let ext = path.extension()?;
    if ext.is_empty() { return None}
    let file = fs::read(path);

    if file.is_err() {
        return None;
    }

    let file_contents = file.unwrap();
    find_prefixes(&file_contents, ns_dict);
    let res = store.load_dataset(Cursor::new(&file_contents), DatasetFormat::TriG, None);
    if res.is_err() {
        println!("Error saving quads to store");
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
    bindings: Vec<Map<String, serde_json::Value>>
}


fn print_query(store: &Store, query: &str, ns_dict: &mut Prefix) {
    let mut writer: Vec<_> = Vec::new();
    let solutions = store.query(query);

    let res = solutions.unwrap().write(&mut writer, QueryResultsFormat::Json);
    if res.is_err() {
        println!("Error in parsing the results");
    }
    let object: SparqlJson = serde_json::from_slice(&writer).expect("Error in Parsing Json");
    let vars = object.head;

    let mut table = Table::new();
    let headings = Row::new(vars.vars.clone().into_iter().map(|x| Cell::new(&x)).collect());
    table.add_row(headings);

    // the following loop should really be placed in its own function
    // perhaps a module and re-write the pretty printing of the table
    for result in object.results.bindings {
        let mut print_res: Vec<Cell> = vec![];
        for var in &vars.vars {
            if let Some(serde_json::Value::Object(var_map)) = &result.get(&var.to_string()).or(None) {
                let rdf_type = &var_map["type"];
                let let_return_value = match rdf_type.as_str() {
                    Some("uri") => {
                        let res = ns_dict.shorten_uri(&var_map["value"].to_string());
                        res
                    },
                    Some("literal") => var_map["value"].to_string(),
                    Some("bnode") => var_map["value"].to_string(),
                    Some("triple") => format!("{}\t{}\t{}", var_map["subject"], var_map["predicate"], var_map["object"]),
                    _ => continue
                };
                print_res.push(Cell::new(&let_return_value));
            }

        }
        table.add_row(Row::new(print_res));

    }
    table.printstd();

}

fn main() {
    let args = Args::parse();

    let mut store = Store::new().unwrap();

    let dir = args.directory.clone();

    let mut ns_dict: Prefix = Prefix::new();

    let paths = fs::read_dir(dir).unwrap();
    for path in paths {
        if path.is_err() {
            println!("Path contains error: {:?}", path);
            continue
        };
        update_store(&mut store, path.unwrap().path(), &mut ns_dict);
    }

    let length = store.len();
    if length.is_err() || length.unwrap() == 0 {
        println!("Error in loading datasets");
        return
    }

    let query = match args.query {
        Some(str) => str,
        None => {
            let q = readlinefn();
            match q {
                Some(str) => str,
                None => panic!("Error in readline")
            }
        },
    };

    if std::path::Path::new(&query).exists()  {
        let read_file = fs::read_to_string(&query);
        if read_file.is_err() {
            println!("There is an error in reading the query file");
            return
        }
        print_query(&store, &read_file.unwrap(), &mut ns_dict);

        return
    }
    // println!("query: {query}");

    print_query(&store, &query, &mut ns_dict);

}
