#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate path_abs;
extern crate dirs;

pub mod desktop;
pub mod cli;
pub mod gui;

fn main() {
    let arg_matches: clap::ArgMatches = clap_app!(myapp =>
        (name: env!("CARGO_PKG_NAME"))
        (version: env!("CARGO_PKG_VERSION"))
        (author:  "Nigel Baillie <metreckk@gmail.com>")
        (about:   "Creates/updates .desktop files in the applications directory with ease")

        (@arg FILE_OR_ENTRY:                                 "Executable file or entry index or entry name")

        (@arg name:        --name        -n   +takes_value   "Name of program")
        (@arg icon:        --icon        -i   +takes_value   "Path to icon")
        (@arg categories:  --categories  -c   +takes_value   "Semicolon-separated categories")
        (@arg path:        --path        -p   +takes_value   "Working directory for when <FILE> gets run (defaults to $PWD)")
        (@arg comment:     --tooltip     -t   +takes_value   "Tooltip when user hovers over application in launcher")
        (@arg yes: -y                                        "Create/update desktop entry without asking about anything")
        (@arg rm: --remove --rm                              "Remove selected entry")
        (@arg status: --status -s                            "View desktop files managed by mkdesktop")
        (@arg gui:    --gui   -g                             "Start GUI")
    ).get_matches();

    let entry = match arg_matches.value_of("FILE_OR_ENTRY") {
        Some(selector) => Some(desktop::select(&selector)),
        None           => None
    };

    if arg_matches.is_present("gui") {
        gui::editor(entry);
    }
    else if arg_matches.is_present("rm") {
        cli::remove(entry);
    }
    else if arg_matches.is_present("status") || !arg_matches.is_present("FILE_OR_ENTRY") {
        cli::status(entry);
    }
    else {
        cli::create_or_update(entry, arg_matches);
    }
}


