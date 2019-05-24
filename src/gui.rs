extern crate gtk;

use gtk::prelude::*;
use gtk::{Window, HeaderBar};

use std::process;

include!(concat!(env!("OUT_DIR"), "/new-entry.glade.rs"));

pub fn begin() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        process::exit(10);
    }

    let builder = gtk::Builder::new_from_string(NEW_ENTRY_GLADE);

    let window: Window = builder.get_object("new_entry_window").expect("New-entry GUI didn't have a window");

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

