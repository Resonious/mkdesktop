extern crate gtk;
extern crate gdk_pixbuf;
extern crate glib;

use gtk::prelude::*;
use gtk::{Window, HeaderBar, FileChooserButton, Image, Label, Button};
use gdk_pixbuf::Pixbuf;
use glib::GString;

use std::io;
use std::process;
use std::path::PathBuf;

use super::desktop;

include!(concat!(env!("OUT_DIR"), "/new-entry.glade.rs"));

const ICON_PREVIEW_SIZE: i32 = 128;


fn set_icon_preview(image: &Image, preview_filename: PathBuf, size: i32) {
    let pixbuf = match Pixbuf::new_from_file_at_scale(
        preview_filename,
        size, size,
        true
    ) {
        Ok(x) => x,
        Err(_e) => {
            image.set_visible(false);
            return;
        }
    };

    image.set_from_pixbuf(Some(&pixbuf));
}


pub fn begin() {
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

    let window:  Window            = builder.get_object("new_entry_window")   .expect("Window not found in GUI resource");
    let chooser: FileChooserButton = builder.get_object("icon_chooser_button").expect("File chooser not found in GUI resource");

    let name_entry: gtk::Entry = builder.get_object("name_entry").expect("Name entry not found in GUI resource");

    let path_entry: FileChooserButton = builder.get_object("path_chooser").expect("Path chooser not found in GUI resource");
    let exec_entry: gtk::Entry = builder.get_object("exec_entry").expect("Exec entry not found in GUI resource");
    let icon_entry: FileChooserButton = builder.get_object("icon_chooser_button").expect("Icon chooser not found in GUI resource");

    let comment_entry: gtk::Entry = builder.get_object("comment_entry").expect("Comment entry not found in GUI resource");
    let categories_entry: gtk::Entry = builder.get_object("categories_entry").expect("Categories entry not found in GUI resource");

    let preview_icon: Image = builder.get_object("preview_icon").expect("Preview icon not found in GUI resource");
    let preview_text: Label = builder.get_object("preview_name").expect("Preview name not found in GUI resource");

    let create_button: Button = builder.get_object("create_button").expect("Create button not found in GUI resource");
    let cancel_button: Button = builder.get_object("cancel_button").expect("Create button not found in GUI resource");


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

    cancel_button.connect_clicked(|button| {
        let toplevel = match button.get_toplevel() {
            Some(top) => top,
            None      => return
        };

        match toplevel.dynamic_cast::<Window>() {
            Ok(window) => window.close(),
            Err(_)     => return
        }
    });

    create_button.connect_clicked(move |_| {
        // TODO actual validation of input
        // TODO don't just spit to stdout
        let mut stdout = io::stdout();

        let name = name_entry.get_text().expect("Please have name");
        let exec = exec_entry.get_text().expect("Please have command");

        let path_path = path_entry.get_filename().expect("Please have name");
        let path = path_path.to_str().expect("Couldn't get string from path");

        let icon_path = icon_entry.get_filename().expect("Please have icon");
        let icon = icon_path.to_str().expect("Couldn't get string from path");

        let comment = comment_entry.get_text();
        let categories = categories_entry.get_text();

        desktop::make_desktop(
            &name,
            &comment.unwrap_or(GString::from("")),
            &path,
            &exec,
            &icon,
            false,
            &categories.unwrap_or(GString::from("")),
            &mut stdout
        ).expect("Couldn't write the damn thing!!! WHY!!!");
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

