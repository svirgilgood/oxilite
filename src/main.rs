use oxigraph::{io::DatasetFormat, store::Store, sparql::{ QueryResults, QueryResultsFormat}};
use std::{fs, path::PathBuf, io::Cursor};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None )]
struct Args {
    /// name of the directory for trig/nq files
    #[arg(short, long)]
    directory: String,

    /// name of the file for loading the query
    #[arg(short, long)]
    query: String,
}


fn update_store(store: &mut Store, path: PathBuf) -> Option<()> {
    let ext = path.extension()?;
    if ext.is_empty() { return None}
    let file = fs::read(path); // read_to_string(path);

    if file.is_err() {
        return None;
    }

    let file_contents = file.unwrap();
    let res = store.load_dataset(Cursor::new(&file_contents), DatasetFormat::TriG, None);
    if res.is_err() {
        println!("Error saving quads to store");
        return None;
    }

    Some(())
}

fn main() {
    let args = Args::parse();

    let mut store = Store::new().unwrap();

    let dir = args.directory.clone();

    let paths = fs::read_dir(dir).unwrap();
    for path in paths {
        if path.is_err() {
            println!("Path contains error: {:?}", path);
            continue
        };
        update_store(&mut store, path.unwrap().path());
    }


    let length = store.len();
    if length.is_err() || length.unwrap() == 0 {
        println!("Error in loading datasets");
        return
    }

    let query = args.query.clone();

    if std::path::Path::new(&query).exists()  {
        let read_file = fs::read_to_string(&query);
        if read_file.is_err() {
            println!("There is an error in reading the query file");
            return
        }
        if let QueryResults::Solutions(solutions) = store.query(&read_file.unwrap()).unwrap() {
            for solution in solutions {
                println!("{:?}", solution.unwrap())
            }
        }
        return
    }
    println!("query: {query}");
    let solutions = store.query(&query);

    let mut writer: Vec<_> = Vec::new();
    if solutions.is_err() {
        println!("Error in evaluating query");

    } else {
        let res = solutions.unwrap().write(&mut writer, QueryResultsFormat::Tsv);
        if res.is_err() {
            println!("Error in parsing the results")
        }
        println!("{}", std::str::from_utf8(&writer).unwrap());
    }
}
