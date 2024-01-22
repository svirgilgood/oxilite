// use rustyline::error::ReadlineError;
use crate::prefix::Prefix;

use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Editor, Completer, Helper, Highlighter, Hinter, Validator};

#[derive(Completer, Helper, Highlighter, Hinter, Validator)]
struct InputValidator {
    #[rustyline(Validator)]
    brackets: MatchingBracketValidator,
    #[rustyline(Highlighter)]
    highlighter: MatchingBracketHighlighter,
}


///
/// Read the function from the command line
/// This function reads a sparql file from a command prompt
/// TODO: Update the rl editor to handle syntax highlighting and multi line commands
pub fn readlinefn(ns_dict: &Prefix) -> Option<String> {
  // matching the
  let helper = InputValidator {
      brackets: MatchingBracketValidator::new(),
      highlighter: MatchingBracketHighlighter::new(),
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
      return None
    }
  }

}


