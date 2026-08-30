// TODO
// name should use filename if gpx name is just a timestamp
// load file should default to current directory and/or most recent

mod app;
mod journey;

#[cfg(not(target_arch = "wasm32"))]
fn init_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();
}

#[cfg(target_arch = "wasm32")]
fn init_logger() {
    console_log::init_with_level(log::Level::Error).expect("failed to initialize console logger");
}

#[cfg(not(target_arch = "wasm32"))]
// native entry point
fn main() -> eframe::Result {
    init_logger();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1800.0, 1000.0])
            .with_min_inner_size([1200.0, 600.0])
            .with_maximized(true)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    let (name, gpx) = journey::import_sample().unwrap();
    log::info!("Loaded sample journey: {}", name);

    eframe::run_native(
        "Journey View",
        options,
        Box::new(|cc| {
            setup_dark_theme(&cc.egui_ctx);
            Ok(Box::new(app::App::new(&cc.egui_ctx, gpx, name, None)))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
// wasm entry point
fn main() {
    use wasm_bindgen::JsCast;
    console_error_panic_hook::set_once();
    init_logger();

    wasm_bindgen_futures::spawn_local(async {
        let window = web_sys::window().expect("no window");

        let document = window.document().expect("no document");

        let canvas = document
            .get_element_by_id("rust-canvas")
            .expect("no canvas found")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");

        let web_options = eframe::WebOptions::default();

        let (mut name, mut gpx) = journey::import_sample().unwrap();
        log::info!("Loaded sample journey in browser: {}", name);

        let mut url = String::new();
        if let Ok(origin) = window.location().origin() {
            url = origin;
        }
        if let Ok(pathname) = window.location().pathname() {
            url.push_str(&pathname);
        }

        if let Ok(search) = window.location().search() {
            if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
                if let Some(journey_string) = params.get("j") {
                    match journey::import(&journey_string) {
                        Ok((qstring_name, qstring_gpx)) => {
                            name = qstring_name.clone();
                            gpx = qstring_gpx.clone();
                            log::info!("Loaded journey from URL: {}", name);
                        }
                        Err(_) => {
                            log::warn!("Failed to decode journey from URL");
                        }
                    }
                }
            }
        };

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    setup_dark_theme(&cc.egui_ctx);
                    Ok(Box::new(app::App::new(&cc.egui_ctx, gpx, name, Some(url))))
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
