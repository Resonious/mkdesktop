use std::io;
use std::path::PathBuf;

pub fn applications_dir() -> PathBuf {
    let mut result = PathBuf::new();

    result.push(dirs::data_dir().expect("Couldn't figure out data directory."));
    result.push("applications/mkdesktop");

    // TODO do this creation elsewhere... before the desktop file is written
    let create_dir = result.clone();
    std::fs::create_dir_all(create_dir).expect("Couldn't create directory to put desktop file in");

    result
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