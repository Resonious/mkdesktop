extern crate regex;
extern crate lazy_static;

use std::io;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use regex::{Regex, RegexBuilder};


#[derive(Clone)]
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


    pub fn filename(&self) -> String {
        name_to_filename(&self.name)
    }

    pub fn filepath(&self) -> PathBuf {
        name_to_desktop_file_path(&self.name)
    }


    pub fn get_name(&self) -> &str { return &self.name; }
    pub fn get_comment(&self) -> &str { return &self.comment; }
    pub fn get_path(&self) -> &str { return &self.path; }
    pub fn get_exec(&self) -> &str { return &self.exec; }
    pub fn get_icon(&self) -> &str { return &self.icon; }
    pub fn get_categories(&self) -> &str { return &self.categories; }


    /// Creates a new entry with the given fields
    pub fn new(
        name: &str,
        comment: &str,
        path: &str,
        exec: &str,
        icon: &str,
        categories: &str,
    ) -> DesktopEntry {
        DesktopEntry {
            name: name.to_string(),
            shortcut_type: String::from("Application"),
            comment: comment.to_string(),
            path: path.to_string(),
            exec: exec.to_string(),
            icon: icon.to_string(),
            terminal: false,
            categories: categories.to_string(),
        }
    }


    /// Parses DesktopEntry from input stream
    pub fn read(input: &mut io::BufRead) -> DesktopEntry {
        lazy_static! {
            static ref CATEGORY_REGEX: Regex = RegexBuilder::new(r"^\[([^\]]+)\]")
                .build().unwrap();

            static ref ATTR_REGEX: Regex = RegexBuilder::new(r"^([^\[#]+)=([^#]+)(#.*)?$")
                .build().unwrap();

            static ref BOOL_REGEX: Regex = RegexBuilder::new(r"true")
                .case_insensitive(true)
                .build().unwrap();
        }

        let mut result = DesktopEntry::blank();
        let mut line = String::new();
        let mut currently_reading_desktop_entry = false;

        while let Ok(bytes_read) = input.read_line(&mut line) {
            if bytes_read == 0 { break }

            match CATEGORY_REGEX.captures(&line) {
                Some(category_cap) => {
                    let category = category_cap.get(1).unwrap().as_str();
                    currently_reading_desktop_entry = category == "Desktop Entry";
                }
                None => {}
            }

            if !currently_reading_desktop_entry { line.clear(); continue }

            let caps = match ATTR_REGEX.captures(&line) {
                Some(c) => c,
                None    => {
                    line.clear();
                    continue;
                }
            };

            let field = caps.get(1).unwrap().as_str().trim();
            let value = caps.get(2).unwrap().as_str().trim();

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
                "{}\n\tcd {}\n\texec {}",
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


    /// Makes sure the DesktopEntry is registered as a shortcut.
    /// This is called by write_to_apps_dir and is probably useless to call directly.
    pub fn save(&self) -> io::Result<()> {
        let filepath = self.filepath();
        let path = match filepath.to_str() {
            Some(s) => s,
            None => return Err(io::Error::new(io::ErrorKind::Other, "Failed to turn path into string"))
        };

        match Command::new("xdg-desktop-menu").args(&["install", path]).status() {
            Ok(_)  => Ok(()),
            Err(e) => return Err(e)
        }
    }


    /// Writes the DesktopEntry to disk and registers it.
    pub fn write_to_apps_dir(&self) -> io::Result<()> {
        let mut path = data_dir();
        path.push(self.filename());

        let mut file = fs::File::create(path)?;
        self.write(&mut file)?;
        self.save()
    }


    pub fn delete(&self) -> io::Result<()> {
        let filename = self.filename();

        // First, uninstall the desktop entry
        let _uninstall = Command::new("xdg-desktop-menu")
            .args(&["uninstall", &filename])
            .status()?;

        // Next, delete the desktop entry
        let mut path = data_dir();
        path.push(filename);
        fs::remove_file(path)
    }
}


pub fn name_to_filename(name: &str) -> String {
    lazy_static! {
        static ref INVALIDS: Regex = RegexBuilder::new(r"[^\w\-\+_]+")
            .build()
            .expect("Failed to compile filename regex");
    }

    let mut result: String = "mkdesktop-".to_string();
    result += &INVALIDS.replace_all(name, "-").to_string();
    result += ".desktop";
    result
}


pub fn name_to_desktop_file_path(name: &str) -> PathBuf {
    let mut path = data_dir();
    path.push(name_to_filename(name));
    path
}


pub fn data_dir() -> PathBuf {
    let mut result = PathBuf::new();

    result.push(dirs::data_dir().expect("Couldn't figure out data directory."));
    result.push("mkdesktop");

    let create_dir = result.clone();
    fs::create_dir_all(create_dir).expect("Couldn't create directory to put desktop file in");

    result
}


pub fn select(selector: &str) -> io::Result<DesktopEntry> {
    lazy_static! {
        static ref INDEX_SELECTOR: Regex = RegexBuilder::new(r"[\(\)\{\}\[\]\s]*(\d+)[\(\)\{\}\[\]\s]*")
            .build().unwrap();
    }

    let entries = read_desktop_files()?;

    //
    // First, see if the selector is just an index
    //
    if let Some(caps) = INDEX_SELECTOR.captures(selector) {
        let index_str = caps.get(1).unwrap().as_str();
        if let Ok(index) = index_str.parse::<usize>() {
            if index < entries.len() {
                return Ok(entries[index].clone());
            }
            else {
                return Err(io::Error::new(io::ErrorKind::NotFound, format!("No entry at index {}", index)))
            }
        }
    }

    //
    // Otherwise, try to match on filename or entry name
    //
    let trimmed_selector = selector.trim();
    for entry in entries {
        if entry.filename() == selector || entry.name == trimmed_selector {
            return Ok(entry);
        }
    }

    Err(io::Error::new(io::ErrorKind::NotFound, format!("Couldn't find entry matching \"{}\"", selector)))
}


pub fn read_desktop_files() -> io::Result<Vec<DesktopEntry>> {
    let mut result = Vec::<DesktopEntry>::new();

    for direntry_result in fs::read_dir(data_dir())? {
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

        result.push(DesktopEntry::read(&mut reader));
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

    // Actions
    output.write_fmt(format_args!("Actions=delete-shortcut\n"))?;

    output.write_fmt(format_args!("\n[Desktop Action delete-shortcut]\n"))?;
    output.write_fmt(format_args!("Name=Delete Shortcut\n"))?;
    output.write_fmt(format_args!("Exec={} --rm \"{}\"\n", env!("CARGO_PKG_NAME"), name))?;

    // Done -- flush output
    output.flush()?;
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
        let desktop_entry = DesktopEntry::read(&mut stream);

        assert_eq!(desktop_entry.shortcut_type, "Application");
        assert_eq!(desktop_entry.comment, "Flash card based learning tool");
        assert_eq!(desktop_entry.path, "/opt/jmemorise");
        assert_eq!(desktop_entry.exec, "jmemorize");
        assert_eq!(desktop_entry.icon, "jmemorize");
        assert_eq!(desktop_entry.terminal, false);
        assert_eq!(desktop_entry.categories, "Education;Languages;Java;");
    }
}
