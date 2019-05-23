extern crate clap;

use std::io;
use std::env;
use std::process;

use path_abs::PathAbs;

use super::desktop;

pub fn begin(arg_matches: clap::ArgMatches) {
    let yes = arg_matches.is_present("yes");

    let filename = match arg_matches.value_of("FILE") {
        Some(f) => f,
        None => {
            println!("Please specify FILE (or pass --help)");
            process::exit(5);
        }
    };

    let exec_path = match PathAbs::new(filename).expect("Couldn't get file path").absolute() {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to open {} - {}", filename, e);
            process::exit(1);
        }
    };

    let exec = String::from(exec_path.as_path().to_str().expect("Failed to turn exec path into string"));

    let name = match arg_matches.value_of("name") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter a name for the desktop entry (required)", None, yes)
    };
    if name.is_empty() {
        println!("A name is required");
        process::exit(1);
    }

    let comment = match arg_matches.value_of("comment") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter a tooltip for the desktop entry", None, yes),
    };

    let categories = match arg_matches.value_of("categories") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter semicolon-separated categories", None, yes),
    };

    let path = match arg_matches.value_of("path") {
        Some(arg) => String::from(arg),
        None      => ask_stdin_for_str("Please enter the working directory for the binary", pwd(), yes),
    };

    let icon = match arg_matches.value_of("icon") {
        Some(arg) => match PathAbs::new(arg).expect("Couldn't get icon path").absolute() {
            Ok(f)  => String::from(f.as_path().to_str().expect("Failed to turn icon path into string")),
            Err(e) => {
                println!("Failed to open {} - {}", arg, e);
                process::exit(2);
            }
        }
        None      => ask_stdin_for_str("Please enter the path to an icon", None, yes),
    };


    // Write desktop file

    let mut stdout = io::stdout();

    desktop::make_desktop(
        &name,
        &comment,
        &path,
        &exec,
        &icon,
        false,
        &categories,
        &mut stdout
    ).expect("Couldn't write the damn thing!!! WHY!!!");
}


pub fn status(_arg_matches: clap::ArgMatches) {
    let desktop_files = match desktop::read_desktop_files() {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to read desktop files: {}", e);
            process::exit(20);
        }
    };

    for entry in desktop_files {
        println!("{}", entry.display());
    }
}


fn pwd() -> Option<String> {
    match env::var_os("PWD") {
        Some(value) => Some(String::from(value.to_str().unwrap_or_default())),
        None => None
    }
}


fn ask_stdin_for_str(msg: &str, default: Option<String>, skip: bool) -> String {
    let default_val = match default {
        Some(default_val) => default_val,
        None              => String::new(),
    };
    if skip { return default_val }

    if !default_val.is_empty() { println!("{} (default={})", msg, default_val); }
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

