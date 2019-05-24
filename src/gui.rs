extern crate gtk;

use gtk::prelude::*;
use gtk::{Window, HeaderBar, FileChooserButton, Image, Label};
use gdk_pixbuf::Pixbuf;

use std::process;
use std::path::PathBuf;

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

    let preview_icon = builder.get_object::<Image>("preview_icon").expect("Preview icon not found in GUI resource");
    let preview_text = builder.get_object::<Label>("preview_name").expect("Preview name not found in GUI resource");


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
            None => return
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
    header_bar.set_show_close_button(true);
    header_bar.set_title("Desktop Launcher Manager");
    header_bar.set_has_subtitle(false);

    window.set_titlebar(Some(&header_bar));

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    //button.connect_clicked(|_| {
    //    println!("Clicked!");
    //});

    gtk::main();
}

