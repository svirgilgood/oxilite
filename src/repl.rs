// use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;


///
/// Read the function from the command line
/// This function reads a sparql file from a command prompt
/// TODO: Update the rl editor to handle syntax highlighting and multi line commands
pub fn readlinefn() -> Option<String> {
  let new_editor = DefaultEditor::new();
  let mut rl = match new_editor {
    Ok(editor) => editor,
    Err(_) => {
      println!("Error in File Reader");
      return None;
    }
  };

  let readline = rl.readline("");
  match readline {
    Ok(line) => return Some(line),
    Err(_) => {
      println!("Error in Reading the Line");
      return None
    }
  }

}


