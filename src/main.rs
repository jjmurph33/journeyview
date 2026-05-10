mod app;
mod journey;

use eframe;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("GPX Viewer")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    let gpx = journey::import_sample();

    let output = journey::export(&gpx);
    println!("{}\n", output);

    eframe::run_native(
        "GPX Viewer",
        options,
        Box::new(move |cc| {
            let app = app::App::new(cc, gpx);
            Ok(Box::new(app))
        }),
    )
}
