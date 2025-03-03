# Oxilite 

## Usage 

```bash
Usage: sparqlite [OPTIONS]

Options:
  -d, --data <DATA>    Name of the directory or file for trig/nq files, argument can be repeated
  -q, --query <QUERY>  Name of the file or string for loading the query
      --print-query    Print the query before executing
      --db <DB>        Use or create a saved database. By specifying the database these will be stored or they will re-use the exiting database
      --toggle-prefix  Toggle prefix injection. For inline queries the default is to inject the prefixes into the query, but for file based queries, the default is to not inject the prefixes
  -h, --help           Print help
  -V, --version        Print version
```
```
