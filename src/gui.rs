extern crate gtk;

use gtk::prelude::*;
use gtk::{Button, Window, WindowType, HeaderBar};

use std::process;

pub fn begin() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        process::exit(10);
    }


    /////////////////////////////////////////////////////////
    //
    //                   MAIN WINDOW
    //
    /////////////////////////////////////////////////////////
    let window = Window::new(WindowType::Toplevel);
    window.set_title("mkdesktop GUI");


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


    /////////////////////////////////////////////////////////
    //
    //                   BOX AND BUTTON
    //
    /////////////////////////////////////////////////////////
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let button = Button::new_with_label("Click me!");
    vbox.add(&button);


    /////////////////////////////////////////////////////////
    //
    //                    WINDOW CHILDREN
    //
    /////////////////////////////////////////////////////////
    window.add(&vbox);
    window.show_all();


    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    button.connect_clicked(|_| {
        println!("Clicked!");
    });

    gtk::main();
}

