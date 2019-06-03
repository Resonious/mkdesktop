extern crate clap;
extern crate gtk;
extern crate gdk_pixbuf;
extern crate glib;
extern crate gio;
extern crate inotify;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Window, Dialog, HeaderBar, FileChooserButton, Image, Label, Button, Continue};
use gdk_pixbuf::Pixbuf;
use glib::GString;
use glib::MainContext;

use inotify::{EventMask, WatchMask, Inotify};

use std::io;
use std::process::{self, Command};
use std::path::Path;
use std::error::Error;
use std::thread;
use std::rc::Rc;
use std::cell::Cell;

use super::desktop::{DesktopEntry, data_dir, read_desktop_files};

include!(concat!(env!("OUT_DIR"), "/new-entry.glade.rs"));
include!(concat!(env!("OUT_DIR"), "/error-dialog.glade.rs"));

include!(concat!(env!("OUT_DIR"), "/list-entries.glade.rs"));
include!(concat!(env!("OUT_DIR"), "/entry.glade.rs"));

const ICON_PREVIEW_SIZE: i32 = 128;


fn set_icon_preview<P: AsRef<Path>>(image: &Image, preview_filename: P, size: i32) {
    let pixbuf = match Pixbuf::new_from_file_at_scale(
        preview_filename,
        size, size,
        false // Preserve aspect ratio
    ) {
        Ok(x) => x,
        Err(_e) => {
            image.set_visible(false);
            return;
        }
    };

    image.set_from_pixbuf(Some(&pixbuf));
}


fn error_dialog(message: &str) -> Dialog {
    let builder = gtk::Builder::new_from_string(ERROR_DIALOG_GLADE);

    let dialog: Dialog = builder.get_object("error_dialog").unwrap();

    let label: Label = builder.get_object("error_message").unwrap();
    label.set_text(message);

    let close_button: Button = builder.get_object("close_error_button").unwrap();
    close_button.connect_clicked(close_window);

    dialog
}


fn setup_list_ui(
    entries_result: io::Result<Vec<DesktopEntry>>,
    entries_container: &gtk::Container,
    deleted_entry: Rc<Cell<Option<DesktopEntry>>>,
    undo_control: &gtk::Button
) {
    /////////////////////////////////////////////////////////
    //
    //                   ERROR CHECKING
    //
    /////////////////////////////////////////////////////////

    let entries = match entries_result {
        Ok(mut e) => {
            e.sort_unstable_by_key(|entry| entry.get_name().to_string());
            e
        }
        Err(error) => error_out(error.description())
    };


    /////////////////////////////////////////////////////////
    //
    //                   POPULATE LIST
    //
    /////////////////////////////////////////////////////////

    for entry in entries {
        let builder = gtk::Builder::new_from_string(ENTRY_GLADE);

        let entry_widget: gtk::Widget = builder.get_object("entry").unwrap();

        let categories_label: Label = builder.get_object("categories_value").unwrap();
        let comment_label:    Label = builder.get_object("comment_value").unwrap();
        let name_label:       Label = builder.get_object("name_value").unwrap();
        let exec_label:       Label = builder.get_object("exec_value").unwrap();
        let path_label:       Label = builder.get_object("path_value").unwrap();

        let icon: Image = builder.get_object("icon_image").unwrap();

        let launch_entry: Button = builder.get_object("launch_entry_button").unwrap();
        let delete_entry: Button = builder.get_object("delete_entry_button").unwrap();
        let edit_entry:   Button = builder.get_object("edit_entry_button").unwrap();

        let undo: gtk::Button = undo_control.clone();

        categories_label.set_text(entry.get_categories());
        comment_label.set_text(entry.get_comment());
        name_label.set_text(entry.get_name());
        exec_label.set_text(entry.get_exec());
        path_label.set_text(entry.get_path());

        set_icon_preview(&icon, entry.get_icon(), ICON_PREVIEW_SIZE);

        // Launch button functionality
        let entry_to_launch = entry.clone();
        launch_entry.connect_clicked(move |_| {
            match Command::new("sh")
                .current_dir(entry_to_launch.get_path())
                .arg("-c")
                .arg(entry_to_launch.get_exec())
                .spawn() {
                    Ok(_) => {}
                    Err(launch_error) => {
                        let dialog = error_dialog(launch_error.description());
                        dialog.show_all();
                        dialog.run();
                    }
                }
            
        });

        // Delete button functionality
        let entry_to_delete = entry.clone();
        let saved_entry = deleted_entry.clone();
        delete_entry.connect_clicked(move |_| {
            match entry_to_delete.delete() {
                Ok(()) => {}
                Err(error) => {
                    let dialog = error_dialog(error.description());
                    dialog.show_all();
                    dialog.run();
                    return;
                }
            }

            saved_entry.set(Some(entry_to_delete.clone()));

            undo.set_visible(true);
        });

        // Edit button functionality
        let entry_to_edit = entry.clone();
        edit_entry.connect_clicked(move |widget| {
            let entry = entry_to_edit.clone();
            editor(&app_of(widget), Some(entry));
        });

        entries_container.add(&entry_widget);
    }
}


pub fn start(entry: Option<io::Result<DesktopEntry>>, arg_matches: clap::ArgMatches) {
    init();

    let app = gtk::Application::new("me.nigelbaillie.mkdesktop", Default::default())
        .expect("Failed to create GTK Application");
    
    glib::set_application_name("mkdesktop");
    gtk::Window::set_default_icon_name("mkdesktop");
    
    if arg_matches.is_present("new") {
        app.connect_activate(|app| editor(app, None));
    }
    else if arg_matches.is_present("status") || !arg_matches.is_present("FILE_OR_ENTRY") {
        app.connect_activate(|app| index(app, read_desktop_files()));
    }
    else {
        match entry {
            Some(result) => match result {
                Ok(e) => {
                    app.connect_activate(move |app| editor(app, Some(e.clone())));
                }
                Err(error_message) => {
                    let dialog = error_dialog(error_message.description());
                    dialog.show_all();
                    dialog.run();
                    process::exit(30);
                }
            },
            None => { app.connect_activate(|app| index(app, read_desktop_files())); }
        }
    }

    app.run(Default::default());
}


pub fn index(app: &gtk::Application, entries_result: io::Result<Vec<DesktopEntry>>)  {
    /////////////////////////////////////////////////////////
    //
    //           CREATE/EXTRACT WIDGETS OF INTEREST
    //
    /////////////////////////////////////////////////////////
    let builder = gtk::Builder::new_from_string(LIST_ENTRIES_GLADE);

    let window:  ApplicationWindow = builder.get_object("window").unwrap();
    let entries_container: gtk::Container = builder.get_object("entries_container").unwrap();
    let new_entry: Button = builder.get_object("new_entry_button").unwrap();
    let undo: Button = builder.get_object("undo_button").unwrap();

    let deleted_entry: Rc<Cell<Option<DesktopEntry>>> = Rc::new(Cell::new(None));

    window.set_application(app);

    new_entry.connect_clicked(|w| {
        editor(&app_of(w), None);
    });

    let deleted_entry_to_restore = deleted_entry.clone();
    undo.connect_clicked(move |button| {
        let entry = deleted_entry_to_restore.clone();

        match entry.take() {
            Some(entry) => entry.write_to_apps_dir().unwrap(),
            None        => error_out("BUG: Undo button was present when it shouldn't have been.")
        }

        button.set_visible(false);
    });

    setup_list_ui(entries_result, &entries_container, deleted_entry.clone(), &undo);

    /////////////////////////////////////////////////////////
    //
    //              REFRESH ON FILE CHANGES
    //
    /////////////////////////////////////////////////////////

    let (tx, rx) = MainContext::channel(glib::PRIORITY_DEFAULT);

    thread::spawn(move || {
        let mut inotify = Inotify::init().expect("Failed to initialize inotify");
        let dir_to_watch = data_dir();

        // If this fails, we'll just panic out of the thread and not get updates
        // (no big deal)
        inotify.add_watch(
            dir_to_watch,
            WatchMask::CREATE | WatchMask::DELETE | WatchMask::MODIFY
        ).expect("Failed to add inotify watch");

        let mut buffer = [0u8; 4096];
        loop {
            let events = inotify
                .read_events_blocking(&mut buffer)
                .expect("Failed to read inotify events");

            // Refresh as long as there was an event on a file
            let mut should_refresh = false;
            for event in events {
                should_refresh = should_refresh || !event.mask.contains(EventMask::ISDIR)
            }

            if should_refresh {
                match tx.send(()) {
                    Ok(_) => {}
                    Err(_) => return
                }
            }
        }
    });

    // Remove all children and re-read desktop files whenever there's a filesystem change
    rx.attach(None, move |_| {
        entries_container.foreach(|child| { child.destroy(); });
        let new_entries = read_desktop_files();
        setup_list_ui(new_entries, &entries_container, deleted_entry.clone(), &undo.clone());
        Continue(true)
    });


    /////////////////////////////////////////////////////////
    //
    //                    SHOW WINDOW
    //
    /////////////////////////////////////////////////////////

    window.show_all();
}


pub fn editor(app: &gtk::Application, entry: Option<DesktopEntry>) {
    let builder = gtk::Builder::new_from_string(NEW_ENTRY_GLADE);


    /////////////////////////////////////////////////////////
    //
    //               EXTRACT WIDGETS OF INTEREST
    //
    /////////////////////////////////////////////////////////

    let window:  Window            = builder.get_object("new_entry_window").unwrap();
    let chooser: FileChooserButton = builder.get_object("icon_chooser_button").unwrap();

    let name_entry: gtk::Entry = builder.get_object("name_entry").unwrap();

    let path_entry: FileChooserButton = builder.get_object("path_chooser").unwrap();
    let exec_entry: gtk::Entry = builder.get_object("exec_entry").unwrap();
    let icon_entry: FileChooserButton = builder.get_object("icon_chooser_button").unwrap();

    let comment_entry: gtk::Entry = builder.get_object("comment_entry").unwrap();
    let categories_entry: gtk::Entry = builder.get_object("categories_entry").unwrap();

    let preview_icon: Image = builder.get_object("preview_icon").unwrap();
    let preview_text: Label = builder.get_object("preview_name").unwrap();

    let create_button: Button = builder.get_object("create_button").unwrap();
    let cancel_button: Button = builder.get_object("cancel_button").unwrap();

    window.set_application(app);


    /////////////////////////////////////////////////////////
    //
    //          INITIAL VALUES AND ERROR CHECKING
    //
    /////////////////////////////////////////////////////////
    
    let mut old_entry_to_delete = None;

    match entry {
        Some(entry) => {
            name_entry.set_text(entry.get_name());
            preview_text.set_text(entry.get_name());
            path_entry.set_filename(entry.get_path());
            exec_entry.set_text(entry.get_exec());
            icon_entry.set_filename(entry.get_icon());
            comment_entry.set_text(entry.get_comment());
            categories_entry.set_text(entry.get_categories());

            old_entry_to_delete = Some(entry);

            create_button.set_label("Save");
        }
        None => {}
    }


    /////////////////////////////////////////////////////////
    //
    //               FILE CHOOSER AND ICON PREVIEWS
    //
    /////////////////////////////////////////////////////////

    chooser.connect_update_preview(|chooser| {
        let preview: Image = match chooser.get_preview_widget() {
            Some(widget) => widget.dynamic_cast().expect("Chooser preview image wasn't an Image"),
            None => return
        };

        let preview_filename = match chooser.get_preview_filename() {
            Some(filename) => filename,
            None => {
                preview.set_visible(false);
                return;
            }
        };

        set_icon_preview(&preview, preview_filename, ICON_PREVIEW_SIZE);
        preview.set_visible(true);
    });

    chooser.connect_selection_changed(move |chooser| {
        let preview_filename = match chooser.get_filename() {
            Some(filename) => filename,
            None           => return
        };

        set_icon_preview(&preview_icon, preview_filename, ICON_PREVIEW_SIZE);
    });

    name_entry.connect_changed(move |entry| {
        match entry.get_text() {
            Some(text) => preview_text.set_text(&text),
            None       => preview_text.set_text("")
        }
    });


    /////////////////////////////////////////////////////////
    //
    //                    HEADER BAR
    //
    /////////////////////////////////////////////////////////

    let header_bar = HeaderBar::new();
    header_bar.set_show_close_button(false);
    if old_entry_to_delete.is_some() {
        header_bar.set_title("Editing Shortcut");
    }
    else {
        header_bar.set_title("New Shortcut");
    }
    header_bar.set_has_subtitle(true);
    header_bar.set_subtitle("mkdesktop");

    header_bar.pack_start(&cancel_button);
    header_bar.pack_end(&create_button);

    window.set_titlebar(Some(&header_bar));


    /////////////////////////////////////////////////////////
    //
    //                   BUTTON EVENTS
    //
    /////////////////////////////////////////////////////////

    cancel_button.connect_clicked(close_window);

    macro_rules! submit {
        () => {
            {
            // Clone components that need to be captured
            let submitted_name = name_entry.clone();
            let submitted_exec = exec_entry.clone();
            let submitted_path = path_entry.clone();
            let submitted_icon = icon_entry.clone();
            let submitted_comment = comment_entry.clone();
            let submitted_categories = categories_entry.clone();
            let to_delete = old_entry_to_delete.clone();
            move |widget| {
                // TODO actual validation of input

                let name = submitted_name.get_text().expect("Please have name");
                let exec = submitted_exec.get_text().expect("Please have command");

                let path_path = submitted_path.get_filename();
                let path = match path_path {
                    Some(val) => Some(String::from(val.to_str().expect("Couldn't get string from path path"))),
                    None      => None
                };

                let icon_path = submitted_icon.get_filename();
                let icon = match icon_path {
                    Some(val) => Some(String::from(val.to_str().expect("Couldn't get string from icon path"))),
                    None      => None
                };

                let comment    = submitted_comment.get_text();
                let categories = submitted_categories.get_text();

                let new_entry = DesktopEntry::new(
                    &name,
                    &comment.unwrap_or(GString::from("")),
                    &path.unwrap_or_default(),
                    &exec,
                    &icon.unwrap_or_default(),
                    &categories.unwrap_or(GString::from(""))
                );
                
                // Write result and save
                match new_entry.write_to_apps_dir() {
                    Ok(()) => {}
                    Err(error) => {
                        error_dialog(error.description());
                        return;
                    }
                };

                // Delete old entry if we were doing an edit and the name changed.
                match &to_delete {
                    Some(old_entry) => {
                        if old_entry.filename() != new_entry.filename() {
                            match old_entry.delete() {
                                Ok(()) => {}
                                Err(error) => {
                                    error_dialog(&format!(
                                        "Error removing old desktop entry: {}\nUnfortunately, that means you're stuck with a duplicate.",
                                        error.description()
                                    ));
                                }
                            }
                        }
                    }
                    None => {}
                }

                window_of(widget).close();
            }
            }
        };
    }

    create_button.connect_clicked(submit!());
    name_entry.connect_activate(submit!());
    exec_entry.connect_activate(submit!());
    comment_entry.connect_activate(submit!());
    categories_entry.connect_activate(submit!());


    /////////////////////////////////////////////////////////
    //
    //                    SHOW WINDOW
    //
    /////////////////////////////////////////////////////////

    window.show_all();
}


fn error_out(error_message: &str) -> ! {
    let dialog = error_dialog(error_message);
    dialog.show_all();
    dialog.run();
    process::exit(30);
}


fn init() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        process::exit(10);
    }
}


fn app_of<W: IsA<gtk::Widget>>(widget: &W) -> gtk::Application {
    let toplevel = widget.get_toplevel().expect("Tried to get window of toplevel-less widget");
    let appwindow = toplevel.dynamic_cast::<ApplicationWindow>().expect("Widget's toplevel was not an ApplicationWindow");
    appwindow.get_application().expect("ApplicationWindow did not have an Application")
}

fn window_of<W: IsA<gtk::Widget>>(widget: &W) -> gtk::ApplicationWindow {
    let toplevel = widget.get_toplevel().expect("Tried to get window of toplevel-less widget");
    toplevel.dynamic_cast::<ApplicationWindow>().expect("Widget's toplevel was not an ApplicationWindow")
}


//
// Events
//

fn close_window(button: &Button) {
    let toplevel = match button.get_toplevel() {
        Some(top) => top,
        None      => return
    };

    match toplevel.dynamic_cast::<Window>() {
        Ok(window) => window.close(),
        Err(_)     => return
    }
}

