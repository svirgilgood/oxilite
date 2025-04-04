// use rustyline::error::ReadlineError;
use crate::prefix::Prefix;

use spargebra::Query;

use rustyline::error::ReadlineError;
use rustyline::validate;
use rustyline::validate::{ValidationContext, ValidationResult};
use rustyline::{Completer, Editor, Helper, Highlighter, Hinter, Validator};

#[derive(Completer, Helper, Highlighter, Hinter, Validator)]
struct InputValidator {
    #[rustyline(Validator)]
    validator: SparqlValidator, //validator: MatchingBracketValidator, //#[rustyline(Highlighter)
                                //highlighter: MatchingBracketHighlighter,
}

#[derive(Default)]
struct SparqlValidator {
    _priv: (),
}

impl SparqlValidator {
    /// constructor
    #[must_use]
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl validate::Validator for SparqlValidator {
    fn validate(
        &self,
        ctx: &mut ValidationContext,
    ) -> Result<rustyline::validate::ValidationResult, ReadlineError> {
        validate_sparql_string(ctx.input())
    }
}

fn validate_sparql_string(input: &str) -> Result<ValidationResult, ReadlineError> {
    let query = Query::parse(input, None);
    //println!("\nline 42: {:?}", &query);
    // The following if needs to be removed in order for the
    // implementation to work
    // if query.is_err() {
    //     // I am not sure: How do you pull the error out and format the query with the error highlighted?
    //     println!("\nline 47: {:?}", &query);
    //     // match query {
    //     //     Err(e) => println!("{}", &e),
    //     //     _ => (),
    //     // }
    //     println!("Here is the error\n");
    //     //panic!("hello");
    // };
    match query {
        Err(_) => return Ok(ValidationResult::Incomplete),
        _ => return Ok(ValidationResult::Valid(None)),
    }
    //return ReadlineError;
}

// #[derive(Default)]
// impl SparqlHighlighter {
//
//
// }

///
/// Read the function from the command line
/// This function reads a sparql file from a command prompt
/// TODO: Update the rl editor to handle syntax highlighting and multi line commands
pub fn readlinefn(ns_dict: &Prefix) -> Option<String> {
    // matching the
    let helper = InputValidator {
        //brackets: MatchingBracketValidator::new(),
        validator: SparqlValidator::new(),
        // highlighter: MatchingBracketHighlighter::new(),
    };

    let new_editor = Editor::new();

    if new_editor.is_err() {
        println!("Error in Creating the editor");
        return None;
    }
    let mut editor = new_editor.unwrap();
    editor.set_helper(Some(helper));

    // editor.set_helper(Some(helper));

    let prefixes = ns_dict.format_for_query();

    let readline = editor.readline(&prefixes);
    match readline {
        Ok(line) => return Some(line),
        Err(_) => {
            println!("Error in Reading the Line");
            return None;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn valid_validation() {
        let valid_query = "SELECT ?s ?p ?o { ?s ?p ?o . }";
        let result = validate_sparql_string(valid_query);
        match result {
            Ok(ValidationResult::Valid(_res)) => {
                assert!(true);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn invalid_validation() {
        let incomplete_query = "SELECT ?s ?p ?o { ?s ?p. }";
        let result = validate_sparql_string(incomplete_query);
        match result {
            Ok(ValidationResult::Incomplete) => assert!(true),
            _ => panic!("Not good"),
        }
    }
    #[test]
    fn second_invalid_validation() {
        let incomplete_query = "SELECT ?s ?p ?o ";
        let result = validate_sparql_string(incomplete_query);
        match result {
            Ok(ValidationResult::Incomplete) => assert!(true),
            _ => panic!("Not good"),
        }
    }
}
