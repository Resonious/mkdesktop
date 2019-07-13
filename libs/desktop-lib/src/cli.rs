extern crate clap;

use std::io;
use std::env;
use std::process;
use std::error::Error;

use path_abs::PathAbs;

use super::desktop::{self, DesktopEntry};


macro_rules! optional_entry_field {
    ( $entry:expr, $getter:ident ) => {
        match &$entry {
            Some(entry) => Some(entry.$getter().to_string()),
            None        => None
        }
    };
    ( $entry:expr, $getter:ident, $default:expr ) => {
        match &$entry {
            Some(entry) => Some(entry.$getter().to_string()),
            None        => $default
        }
    };
}


pub fn create_or_update(entry_result: Option<io::Result<DesktopEntry>>, arg_matches: clap::ArgMatches) {
    let yes   = arg_matches.is_present("yes");

    let (filename, entry) = match entry_result {
        Some(result) => match result {
            // An entry was successfully selected -- we want to update that entry
            Ok(e)  => (e.get_exec().to_string(), Some(e.clone())),

            // An entry couldn't be parsed from what was given -- it must be a filename for a new entry
            Err(_) => match arg_matches.value_of("FILE_OR_ENTRY") {
                Some(f) => (f.to_string(), None),
                None    => error_out("Please specify a file or an entry (see --help)")
            }
        }

        // No file parameter was specified
        None => error_out("Please specify a file or an entry (see --help)")
    };

    // TODO this breaks when loading up an existing entry
    let exec_path = match PathAbs::new(&filename).expect("Couldn't get file path").absolute() {
        Ok(f)  => f,
        Err(e) => error_out(&format!("Failed to open {} - {}", &filename, e))
    };

    let exec = String::from(exec_path.as_path().to_str().expect("Failed to turn exec path into string"));

    let name = match arg_matches.value_of("name") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter a name for the desktop entry (required)", optional_entry_field!(entry, get_name), yes)
    };
    if name.is_empty() {
        println!("A name is required");
        process::exit(1);
    }

    let comment = match arg_matches.value_of("comment") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter a tooltip for the desktop entry", optional_entry_field!(entry, get_comment), yes),
    };

    let categories = match arg_matches.value_of("categories") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter semicolon-separated categories", optional_entry_field!(entry, get_categories), yes),
    };

    let path = match arg_matches.value_of("path") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter the working directory for the binary", optional_entry_field!(entry, get_path, pwd()), yes),
    };

    let icon = match arg_matches.value_of("icon") {
        Some(arg) => match PathAbs::new(arg).expect("Couldn't get icon path").absolute() {
            Ok(f)  => String::from(f.as_path().to_str().expect("Failed to turn icon path into string")),
            Err(e) => error_out(&format!("Failed to open {} - {}", arg, e))
        }
        None => ask_stdin_for_str("Please enter the path to an icon", optional_entry_field!(entry, get_icon), yes),
    };

    // Prepare new entry
    let new_entry = DesktopEntry::new(
        &name,
        &comment,
        &path,
        &exec,
        &icon,
        &categories,
    );

    // Write to disk
    match new_entry.write_to_apps_dir() {
        Ok(()) => {}
        Err(error) => error_out(error.description())
    }

    // Delete old entry file if name was changed
    match entry {
        Some(old_entry) => if old_entry.filename() != new_entry.filename() {
            match old_entry.delete() {
                Ok(()) => {}
                Err(error) => error_out(&format!("Failed to delete old entry ({}) you probably have a duplicate now", error.description()))
            }
        },
        None => {}
    }
}


pub fn status(entry_result: Option<io::Result<DesktopEntry>>) {
    match valid_entry_or_none(entry_result) {
        //
        // When an entry is selected,
        //   print the whole desktop file to STDOUT
        //
        Some(entry) => {
            let mut stdout = io::stdout();
            println!("# {:?}", entry.filepath());
            match entry.write(&mut stdout) {
                Ok(()) => {}
                Err(error) => {
                    println!("Failed to print entry to stdout: {}", error.description());
                    process::exit(21);
                }
            }
        }
        //
        // When no entry is selected,
        //   print an overview of every desktop entry managed by mkdesktop
        //
        None => {
            let desktop_files = match desktop::read_desktop_files() {
                Ok(x) => x,
                Err(e) => {
                    println!("Failed to read desktop files: {}", e);
                    process::exit(20);
                }
            };

            let mut i = 0;
            for entry in desktop_files {
                println!("({}) {}", i, entry.display());
                i += 1;
            }
        }
    }
}


pub fn remove(entry_result: Option<io::Result<DesktopEntry>>) {
    match valid_entry_or_none(entry_result) {
        Some(entry) => match entry.delete() {
            Ok(()) => {}
            Err(error) => {
                println!("Failed to delete entry \"{}\" - {}", entry.get_name(), error.description());
                process::exit(12);
            }
        }
        None => error_out("Please specify an entry, either by index or by name")
    }
}


fn pwd() -> Option<String> {
    match env::var_os("PWD") {
        Some(value) => Some(String::from(value.to_str().unwrap_or_default())),
        None => None
    }
}


fn valid_entry_or_none(entry: Option<io::Result<DesktopEntry>>) -> Option<DesktopEntry> {
    match entry {
        Some(result) => match result {
            Ok(entry) => Some(entry),
            Err(error) => error_out(error.description())
        }
        None => None
    }
}


fn error_out(error: &str) -> ! {
    println!("{}", error);
    process::exit(11);
}


fn ask_stdin_for_str(msg: &str, default: Option<String>, skip: bool) -> String {
    let default_val = match default {
        Some(default_val) => default_val,
        None              => String::new(),
    };
    if skip { return default_val }

    if !default_val.is_empty() { println!("{} (blank='{}')", msg, default_val); }
    else                       { println!("{}", msg); }

    let mut user_input = String::new();

    io::stdin().read_line(&mut user_input)
        .expect("Failed to read from stdin. Pass '-y' to skip checks.");
    
    user_input = String::from(user_input.trim());

    if user_input.is_empty() {
        default_val
    }
    else {
        user_input
    }
}

