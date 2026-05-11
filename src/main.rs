// cargo run for native
// trunk serve --open for web

mod app;
mod journey;

use eframe;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    let gpx = journey::import_sample();
    let mut name = String::from("My Journey");
    if let Some(metadata) = &gpx.metadata {
        if let Some(gpx_name) = &metadata.name {
            name = gpx_name.clone();
        }
    }

    eframe::run_native(
        "Journey View",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new(gpx, name)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        let window = web_sys::window().expect("no window");

        let document = window.document().expect("no document");

        let canvas = document
            .get_element_by_id("rust-canvas")
            .expect("no canvas found")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");

        let web_options = eframe::WebOptions::default();

        let gpx = journey::import_sample();
        let mut name = String::from("My Journey");
        if let Some(metadata) = &gpx.metadata {
            if let Some(gpx_name) = &metadata.name {
                name = gpx_name.clone();
            }
        }

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|_cc| Ok(Box::new(app::App::new(gpx, name)))),
            )
            .await
            .expect("failed to start");
    });
}
