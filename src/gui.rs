extern crate gtk;
extern crate gdk_pixbuf;
extern crate glib;
extern crate gio;
extern crate inotify;

use gtk::prelude::*;
use gtk::{Window, Dialog, HeaderBar, FileChooserButton, Image, Label, Button, Continue};
use gdk_pixbuf::Pixbuf;
use glib::GString;
use glib::MainContext;

use inotify::{EventMask, WatchMask, Inotify};

use std::io;
use std::process;
use std::path::Path;
use std::error::Error;
use std::thread;

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


fn setup_list_ui(entries_result: io::Result<Vec<DesktopEntry>>, entries_container: &gtk::Container) {
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

    // TODO factor this out into a method that's also called on filesystem changes
    for entry in entries {
        let builder = gtk::Builder::new_from_string(ENTRY_GLADE);

        let entry_widget: gtk::Widget = builder.get_object("entry").unwrap();

        let categories_label: Label = builder.get_object("categories_value").unwrap();
        let comment_label:    Label = builder.get_object("comment_value").unwrap();
        let name_label:       Label = builder.get_object("name_value").unwrap();

        let exec_buffer: gtk::TextBuffer = builder.get_object("exec_buffer").unwrap();
        let path_buffer: gtk::TextBuffer = builder.get_object("path_buffer").unwrap();

        let icon: Image = builder.get_object("icon_image").unwrap();

        let launch_entry: Button = builder.get_object("launch_entry_button").unwrap();
        let delete_entry: Button = builder.get_object("delete_entry_button").unwrap();
        let edit_entry: Button = builder.get_object("edit_entry_button").unwrap();

        categories_label.set_text(entry.get_categories());
        comment_label.set_text(entry.get_comment());
        name_label.set_text(entry.get_name());

        exec_buffer.set_text(entry.get_exec());
        path_buffer.set_text(entry.get_path());

        set_icon_preview(&icon, entry.get_icon(), ICON_PREVIEW_SIZE);

        // Launch button functionality
        let entry_to_launch = entry.clone();
        launch_entry.connect_clicked(move |_| {
            // TODO
            println!("TODO: Launch {:?}", entry_to_launch.get_name());
        });

        // Delete button functionality
        let entry_to_delete = entry.clone();
        delete_entry.connect_clicked(move |_| {
            match entry_to_delete.delete() {
                Ok(()) => {}
                Err(error) => error_out(error.description())
            }
        });

        // Delete button functionality
        let entry_to_edit = entry.clone();
        edit_entry.connect_clicked(move |_| {
            let entry = entry_to_edit.clone();
            editor(Some(Ok(entry)));
            process::exit(0);
        });

        entries_container.add(&entry_widget);
    }
}


pub fn index(entries_result: io::Result<Vec<DesktopEntry>>)  {
    init();


    /////////////////////////////////////////////////////////
    //
    //           CREATE/EXTRACT WIDGETS OF INTEREST
    //
    /////////////////////////////////////////////////////////
    let builder = gtk::Builder::new_from_string(LIST_ENTRIES_GLADE);

    let window:  Window = builder.get_object("window").unwrap();
    let entries_container: gtk::Container = builder.get_object("entries_container").unwrap();
    let new_entry: Button = builder.get_object("new_entry_button").unwrap();

    new_entry.connect_clicked(|_| {
        editor(None);
    });

    setup_list_ui(entries_result, &entries_container);

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
        setup_list_ui(new_entries, &entries_container);
        Continue(true)
    });


    /////////////////////////////////////////////////////////
    //
    //                    SHOW WINDOW
    //
    /////////////////////////////////////////////////////////

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}


pub fn editor(entry: Option<io::Result<DesktopEntry>>) {
    init();

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


    /////////////////////////////////////////////////////////
    //
    //          INITIAL VALUES AND ERROR CHECKING
    //
    /////////////////////////////////////////////////////////
    
    let mut old_entry_to_delete = None;

    match entry {
        Some(result) => match result {
            Ok(entry) => {
                name_entry.set_text(entry.get_name());
                preview_text.set_text(entry.get_name());
                path_entry.set_filename(entry.get_path());
                exec_entry.set_text(entry.get_exec());
                icon_entry.set_filename(entry.get_icon());
                comment_entry.set_text(entry.get_comment());
                categories_entry.set_text(entry.get_categories());

                old_entry_to_delete = Some(entry);
            }
            Err(error_message) => {
                let dialog = error_dialog(error_message.description());
                dialog.show_all();
                dialog.run();
                process::exit(30);
            }
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
    //                   BUTTON EVENTS
    //
    /////////////////////////////////////////////////////////

    cancel_button.connect_clicked(close_window);

    create_button.connect_clicked(move |_| {
        // TODO actual validation of input

        let name = name_entry.get_text().expect("Please have name");
        let exec = exec_entry.get_text().expect("Please have command");

        let path_path = path_entry.get_filename();
        let path = match path_path {
            Some(val) => Some(String::from(val.to_str().expect("Couldn't get string from path path"))),
            None      => None
        };

        let icon_path = icon_entry.get_filename();
        let icon = match icon_path {
            Some(val) => Some(String::from(val.to_str().expect("Couldn't get string from icon path"))),
            None      => None
        };

        let comment = comment_entry.get_text();
        let categories = categories_entry.get_text();

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
        match &old_entry_to_delete {
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

        // TODO we want to actually show that it succeeded or something.
    });


    /////////////////////////////////////////////////////////
    //
    //                    HEADER BAR
    //
    /////////////////////////////////////////////////////////

    let header_bar = HeaderBar::new();
    header_bar.set_show_close_button(false);
    header_bar.set_title("Desktop Launcher Manager");
    header_bar.set_has_subtitle(false);

    header_bar.pack_start(&cancel_button);
    header_bar.pack_end(&create_button);

    window.set_titlebar(Some(&header_bar));


    /////////////////////////////////////////////////////////
    //
    //                    SHOW WINDOW
    //
    /////////////////////////////////////////////////////////

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
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

