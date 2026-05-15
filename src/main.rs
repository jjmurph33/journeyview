// cargo run for native
// trunk serve --open for web

mod app;
mod journey;

use eframe;
use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1800.0, 800.0])
            .with_min_inner_size([1200.0, 600.0])
            .with_maximized(true)
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
        Box::new(|cc| {
            setup_dark_theme(&cc.egui_ctx);
            Ok(Box::new(app::App::new(gpx, name)))
        }),
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
                Box::new(|cc| {
                    setup_dark_theme(&cc.egui_ctx);
                    Ok(Box::new(app::App::new(gpx, name)))
                }),
            )
            .await
            .expect("failed to start");
    });
}

fn setup_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(20, 20, 25);
    visuals.window_fill = egui::Color32::from_rgb(25, 25, 30);
    visuals.extreme_bg_color = egui::Color32::from_rgb(15, 15, 20);
    visuals.faint_bg_color = egui::Color32::from_rgb(40, 40, 50);
    visuals.weak_text_color = Some(egui::Color32::from_rgb(200, 200, 200));
    visuals.override_text_color = Some(egui::Color32::from_rgb(240, 240, 245));
    ctx.set_visuals(visuals);
}
