extern crate gtk;

use gtk::prelude::*;
use gtk::{Window, HeaderBar, FileChooser, Image};
use gdk_pixbuf::Pixbuf;

use std::process;

include!(concat!(env!("OUT_DIR"), "/new-entry.glade.rs"));

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

    let window:  Window      = builder.get_object("new_entry_window").expect("Window not found in GUI resource");
    let chooser: FileChooser = builder.get_object("icon_chooser")    .expect("File chooser not found in GUI resource");


    /////////////////////////////////////////////////////////
    //
    //               FILE CHOOSER AND ICON PREVIEW
    //
    /////////////////////////////////////////////////////////

    chooser.connect_selection_changed(|chooser| {
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

        let pixbuf = match Pixbuf::new_from_file_at_scale(
            preview_filename,
            128, 128,
            false
        ) {
            Ok(x) => x,
            Err(_e) => {
                preview.set_visible(false);
                return;
            }
        };

        preview.set_from_pixbuf(Some(&pixbuf));
        preview.set_visible(true);
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

