extern crate gtk;
extern crate gdk_pixbuf;
extern crate glib;

use gtk::prelude::*;
use gtk::{Window, Dialog, HeaderBar, FileChooserButton, Image, Label, Button};
use gdk_pixbuf::Pixbuf;
use glib::GString;

use std::io;
use std::fs;
use std::process;
use std::path::PathBuf;
use std::error::Error;

use super::desktop::{self, DesktopEntry};

include!(concat!(env!("OUT_DIR"), "/new-entry.glade.rs"));
include!(concat!(env!("OUT_DIR"), "/error-dialog.glade.rs"));

const ICON_PREVIEW_SIZE: i32 = 128;


fn set_icon_preview(image: &Image, preview_filename: PathBuf, size: i32) {
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


// TODO make a GUI index of entries that allows create/update/destroy


pub fn editor(entry: Option<io::Result<DesktopEntry>>) {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        process::exit(10);
    }

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
                return;
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


        let mut file = match fs::File::create(desktop::name_to_desktop_file_path(&name)) {
            Ok(f) => f,
            Err(error) => {
                error_dialog(error.description());
                return;
            }
        };

        match desktop::make_desktop(
            &name,
            &comment.unwrap_or(GString::from("")),
            &path.unwrap_or_default(),
            &exec,
            &icon.unwrap_or_default(),
            false,
            &categories.unwrap_or(GString::from("")),
            &mut file
        ) {
            Ok(()) => {}
            Err(error) => {
                error_dialog(error.description());
                return;
            }
        }

        match &old_entry_to_delete {
            Some(entry) => {
                if entry.get_name() != name {
                    match entry.delete() {
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

