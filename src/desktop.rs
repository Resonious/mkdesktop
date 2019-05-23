extern crate regex;
extern crate lazy_static;

use std::io;
use std::fs;
use std::path::PathBuf;

use regex::{Regex, RegexBuilder};


pub struct DesktopEntry {
    name: String,
    shortcut_type: String,
    comment: String,
    path: String,
    exec: String,
    icon: String,
    terminal: bool,
    categories: String,
}

impl DesktopEntry {
    /// Blank DesktopEntry
    pub fn blank() -> DesktopEntry {
        DesktopEntry {
            name: String::new(),
            shortcut_type: String::from("Application"),
            comment: String::new(),
            path: String::new(),
            exec: String::new(),
            icon: String::new(),
            terminal: false,
            categories: String::new(),
        }
    }


    /// Parses DesktopEntry from input stream
    pub fn new(input: &mut io::BufRead) -> DesktopEntry {
        lazy_static! {
            static ref DESKTOP_ATTR: Regex = RegexBuilder::new(r"^([^\[#]+)=([^#]+)(#.*)?$")
                .build()
                .expect("Failed to compile DESKTOP_ATTR regex");

            static ref BOOL_REGEX: Regex = RegexBuilder::new(r"true")
                .case_insensitive(true)
                .build()
                .expect("Failed to compile BOOL_REGEX");
        }

        let mut result = DesktopEntry::blank();
        let mut line = String::new();

        while let Ok(bytes_read) = input.read_line(&mut line) {
            if bytes_read == 0 { break }

            let caps = match DESKTOP_ATTR.captures(&line) {
                Some(c) => c,
                None    => {
                    line.clear();
                    continue;
                }
            };

            let field = caps.get(1).unwrap().as_str().trim();
            let value = caps.get(2).unwrap().as_str().trim();

            println!("Got a cap!    field='{}'   value='{}'", field, value);

            match field {
                "Type"       => { result.shortcut_type = String::from(value) }
                "Name"       => { result.name          = String::from(value) }
                "Comment"    => { result.comment       = String::from(value) }
                "Path"       => { result.path          = String::from(value) }
                "Exec"       => { result.exec          = String::from(value) }
                "Icon"       => { result.icon          = String::from(value) }
                "Terminal"   => { result.terminal      = BOOL_REGEX.captures(&value).is_some() }
                "Categories" => { result.categories    = String::from(value) }
                _ => {}
            }

            line.clear();
        }

        result
    }


    /// Gets you a nice string representation (doesn't include all info)
    pub fn display(&self) -> String {
        if self.path.is_empty() {
            format!(
                "{}\n\texec {}",
                self.name,
                self.exec
            )
        }
        else {
            format!(
                "{}\n\tin {}\n\texec {}",
                self.name,
                self.path,
                self.exec
            )
        }
    }


    pub fn write(&self, output: &mut io::Write) -> io::Result<()> {
        make_desktop(
            &self.name,
            &self.comment,
            &self.path,
            &self.exec,
            &self.icon,
            self.terminal,
            &self.categories,
            output
        )
    }
}


pub fn applications_dir() -> PathBuf {
    let mut result = PathBuf::new();

    result.push(dirs::data_dir().expect("Couldn't figure out data directory."));
    result.push("applications/mkdesktop");

    let create_dir = result.clone();
    fs::create_dir_all(create_dir).expect("Couldn't create directory to put desktop file in");

    result
}


pub fn read_desktop_files() -> io::Result<Vec<DesktopEntry>> {
    let mut result = Vec::<DesktopEntry>::new();

    for direntry_result in fs::read_dir(applications_dir())? {
        let direntry = match direntry_result {
            Ok(x) => x,
            Err(e) => {
                println!("ERROR READING DESKTOP FILE: {}", e);
                continue;
            }
        };

        let file = match fs::File::open(direntry.path()) {
            Ok(x) => x,
            Err(e) => {
                println!("Couldn't open {:?} - {}", direntry.path(), e);
                continue;
            }
        };
        let mut reader = io::BufReader::new(file);

        result.push(DesktopEntry::new(&mut reader));
    }

    Ok(result)
}


pub fn make_desktop(
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


#[cfg(test)]
mod test {
    use super::DesktopEntry;
    use std::io;

    #[test]
    fn desktop_entry_can_parse_desktop_format() {
        let desktop_string = "[Desktop Entry]
# Taken from https://wiki.archlinux.org/index.php/desktop_entries
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
Categories=Education;Languages;Java;";
        let mut stream = io::Cursor::new(desktop_string);
        let desktop_entry = DesktopEntry::new(&mut stream);

        assert_eq!(desktop_entry.shortcut_type, "Application");
        assert_eq!(desktop_entry.comment, "Flash card based learning tool");
        assert_eq!(desktop_entry.path, "/opt/jmemorise");
        assert_eq!(desktop_entry.exec, "jmemorize");
        assert_eq!(desktop_entry.icon, "jmemorize");
        assert_eq!(desktop_entry.terminal, false);
        assert_eq!(desktop_entry.categories, "Education;Languages;Java;");
    }
}
