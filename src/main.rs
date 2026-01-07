// Copyright 2026 SM6WJM

//use glib::Propagation;
use gtk4::Application;
use gtk4::gdk;
//use gtk4::gdk::Display;
use gtk4::gdk::prelude::DisplayExt;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

/// Activate the application
fn activate(application: &gtk4::Application) {
    // Get monitors
    let monitors = gdk::Display::default().unwrap().monitors();
    println!("Number of monitors: {}", monitors.n_items());
    let monitor = monitors.item(0).unwrap();
    let monitor = monitor.downcast::<gdk::Monitor>().unwrap();

    println!(
        "Monitor geometry: x={}, y={}, width={}, height={}",
        monitor.geometry().x(),
        monitor.geometry().y(),
        monitor.geometry().width(),
        monitor.geometry().height()
    );

    // Create a normal window
    let window = gtk4::ApplicationWindow::new(application);
    // Before the window is first realized, set it up to be a layer surface
    window.init_layer_shell();
    // Display above normal windows
    window.set_layer(Layer::Background);
    // Set the window to be on the specified monitor
    window.set_monitor(Some(&monitor));
    // Anchor to all edges of the screen
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);

    // window.set_margin_bottom(0);
    // window.set_margin_top(0);
    // window.set_margin_start(0);
    // window.set_margin_end(0);

    // Don't reserve space (wallpaper should not push windows)
    window.set_exclusive_zone(-1);

    window.set_focusable(false);

    // Load image as a texture
    let file = gio::File::for_path("/home/albin/src/wjmclock/images/mercator.jpg");
    let texture = gdk::Texture::from_file(&file).expect("Failed to load image");

    // Picture scales automatically
    let picture = gtk4::Picture::for_paintable(&texture);
    picture.set_hexpand(true);
    picture.set_vexpand(true);

    // Choose how it scales:
    // - Contain: whole map visible, may add borders
    // - Cover: fills screen, may crop edges (nice for wallpaper)
    picture.set_content_fit(gtk4::ContentFit::Cover);

    window.set_child(Some(&picture));

    // Show the window
    window.present();
}

fn main() {
    println!("Starting clock program.");

    let application = Application::new(Some("se.sm6wjm.wjmclock"), Default::default());

    application.connect_activate(activate);

    application.run();
}
