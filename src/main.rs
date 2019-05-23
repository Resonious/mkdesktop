#[macro_use]
extern crate clap;
extern crate path_abs;

use std::io;
use std::process;
use path_abs::PathAbs;

fn main() {
    let arg_matches: clap::ArgMatches = clap_app!(myapp =>
        (version: env!("CARGO_PKG_VERSION"))
        (author:  "Nigel Baillie <metreckk@gmail.com>")
        (about:   "Creates/updates .desktop files in the appropriate location with ease")

        (@arg FILE:                           +required      "Executable file")

        (@arg name:        --name        -n   +takes_value   "Name of program")
        (@arg icon:        --icon        -i   +takes_value   "Path to icon")
        (@arg categories:  --categories  -c   +takes_value   "Semicolon-separated categories")
        (@arg path:        --path        -p   +takes_value   "Working directory for when <FILE> gets run (defaults to $PWD)")
        (@arg comment:     --tooltip     -t   +takes_value   "Tooltip when user hovers over application in launcher")
        (@arg status:      --status      -s                  "See if a desktop entry already exists for the given file")
        (@arg yes: -y                                        "Create/update desktop entry without asking about anything")
        (@arg verbose: --verbose -v                          "Print extra garbage for debugging")
    ).get_matches();

    // Gather information

    let yes = arg_matches.is_present("yes");

    let filename = arg_matches.value_of("FILE").unwrap();

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
        None      => ask_stdin_for_str("Please enter the working directory for the binary", Some(env!("PWD")), yes),
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

    make_desktop(
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

fn ask_stdin_for_str(msg: &str, default: Option<&str>, skip: bool) -> String {
    if skip { return String::from(default.unwrap_or_default()) }

    match default {
        Some(default_val) => println!("{} (default={})", msg, default_val),
        None              => println!("{}", msg),
    };

    let mut user_input = String::new();

    io::stdin().read_line(&mut user_input)
        .expect("Failed to read from stdin. Pass '-y' to skip checks.");
    
    user_input = String::from(user_input.trim());

    if user_input.is_empty() {
        String::from(default.unwrap_or_default())
    }
    else {
        user_input
    }
}

fn make_desktop(
    name: &str,
    comment: &str,
    path: &str,
    exec: &str,
    icon: &str,
    terminal: bool,
    categories: &str,
    output: &mut io::Write
) -> io::Result<()> {
    /*
    [Desktop Entry]
    # The type as listed above
    Type=Application
    # The version of the desktop entry specification to which this file complies
    Version=1.0
    # The name of the application
    Name=jMemorize
    # A comment which can/will be used as a tooltip
    Comment=Flash card based learning tool
    # The path to the folder in which the executable is run
    Path=/opt/jmemorise
    # The executable of the application, possibly with arguments.
    Exec=jmemorize
    # The name of the icon that will be used to display this entry
    Icon=jmemorize
    # Describes whether this application needs to be run in a terminal or not
    Terminal=false
    # Describes the categories in which this entry should be shown
    Categories=Education;Languages;Java;
    */
    output.write_fmt(format_args!("[Desktop Entry]\n"))?;
    output.write_fmt(format_args!("Type=Application\n"))?;
    output.write_fmt(format_args!("Version=1.0\n"))?;
    output.write_fmt(format_args!("Name={}\n", name))?;
    output.write_fmt(format_args!("Exec={}\n", exec))?;

    if !comment.is_empty()    { output.write_fmt(format_args!("Comment={}\n", comment))?       }
    if !path.is_empty()       { output.write_fmt(format_args!("Path={}\n", path))?             }
    if !icon.is_empty()       { output.write_fmt(format_args!("Icon={}\n", icon))?             }
    if !categories.is_empty() { output.write_fmt(format_args!("Categories={}\n", categories))? }

    if terminal { output.write_fmt(format_args!("Terminal=true\n"))? }
    else        { output.write_fmt(format_args!("Terminal=false\n"))? }

    Ok(())
}
