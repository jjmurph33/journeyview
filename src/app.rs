use eframe::egui;
use egui::{CentralPanel, Color32, Panel, Ui};
use egui_plot::{Line, Plot, PlotPoints};
use gpx::Gpx;

use crate::journey;

enum Mode {
    NORMAL,
    LOAD,
    IMPORT,
    EXPORT,
}

pub struct App {
    gpx: Gpx,
    distance: f64,       // miles
    min_elevation: f64,  // feet
    max_elevation: f64,  // feet
    diff_elevation: f64, // feet
    name: String,
    name_editing: bool,
    name_buffer: String,
    load_buffer: String,
    import_buffer: String,
    status_text: String,
    show_map: bool,   // toggle between map and elevation plot
    reset_plot: bool, // reset plot zoom/pan
    mode: Mode,
}

impl App {
    pub fn new(gpx: Gpx, name: String) -> Self {
        let distance = km_to_mi(distance(&gpx));
        let min_elevation = m_to_ft(min_elevation(&gpx));
        let max_elevation = m_to_ft(max_elevation(&gpx));
        let diff_elevation = max_elevation - min_elevation;
        Self {
            gpx,
            name,
            distance,
            min_elevation,
            max_elevation,
            diff_elevation,
            name_editing: false,
            name_buffer: String::new(),
            load_buffer: String::new(),
            import_buffer: String::new(),
            status_text: String::new(),
            show_map: true,
            reset_plot: false,
            mode: Mode::NORMAL,
        }
    }

    fn top_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                //////////// Name label ////////////////////////
                if self.name_editing {
                    if self.name_buffer.is_empty() {
                        self.name_buffer = self.name.clone();
                    }
                    let resp = ui.text_edit_singleline(&mut self.name_buffer);
                    let commit =
                        ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) || resp.lost_focus();
                    if commit {
                        self.name = self.name_buffer.clone();
                        self.name_editing = false;
                    }
                } else {
                    let lbl = ui.add(
                        egui::Label::new(
                            egui::RichText::new(self.name.clone())
                                .size(28.0)
                                .color(Color32::from_rgb(200, 220, 255))
                                .strong(),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if lbl.clicked() {
                        self.name_editing = true;
                        self.name_buffer = self.name.clone();
                    }
                }
                ///////////////////// Info labels /////////////////////
                ui.label(
                    egui::RichText::new(format!("Distance: {:.1}mi", self.distance))
                        .size(18.0)
                        .color(Color32::from_rgb(210, 210, 220)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Elevation: {:.0}ft -> {:.0}ft ({:.0}ft change)",
                        self.min_elevation, self.max_elevation, self.diff_elevation
                    ))
                    .size(18.0)
                    .color(Color32::from_rgb(210, 210, 220)),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ///////////////////// Map/Elevation button /////////////////////
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(if self.show_map { "Elevation" } else { "Map" })
                                .size(16.0)
                                .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .min_size(egui::Vec2::new(150.0, 50.0))
                        .fill(Color32::from_rgb(76, 175, 80)) // Green
                        .stroke(egui::Stroke::new(2.0, Color32::from_rgb(56, 142, 60))),
                    )
                    .clicked()
                {
                    self.show_map = !self.show_map;
                }
                ui.add_space(10.0);
                ///////////////////// Export button ////////////////////////
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Export")
                                .size(16.0)
                                .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .min_size(egui::Vec2::new(150.0, 50.0))
                        .fill(Color32::from_rgb(33, 150, 243)) // Blue
                        .stroke(egui::Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                    )
                    .clicked()
                {
                    self.mode = Mode::EXPORT;
                }
                ///////////////////// Import button ////////////////////////
                ui.add_space(10.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Import")
                                .size(16.0)
                                .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .min_size(egui::Vec2::new(150.0, 50.0))
                        .fill(Color32::from_rgb(33, 150, 243)) // Blue
                        .stroke(egui::Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                    )
                    .clicked()
                {
                    self.mode = Mode::IMPORT;
                    if let Some(clipboard_text) = read_clipboard() {
                        self.import_buffer = clipboard_text;
                    } else {
                        self.import_buffer.clear();
                    }
                }
                ///////////////////// Load button ////////////////////////
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Load File")
                                .size(16.0)
                                .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .min_size(egui::Vec2::new(150.0, 50.0))
                        .fill(Color32::from_rgb(33, 150, 243)) // Blue
                        .stroke(egui::Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                    )
                    .clicked()
                {
                    self.mode = Mode::LOAD;
                    self.load_buffer.clear();
                }
            });
        });
    }

    fn bottom_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status_text);
        });
    }

    fn load_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Enter file path:");
        ui.text_edit_singleline(&mut self.load_buffer);
        ui.horizontal(|ui| {
            if ui.button("Load").clicked() {
                if !self.load_buffer.is_empty() {
                    self.load_file(self.load_buffer.clone());
                    self.load_buffer.clear();
                }
            }
            if ui.button("Cancel").clicked() {
                self.mode = Mode::NORMAL;
                self.load_buffer.clear();
            }
        });
    }

    fn import_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Paste the encoded string:");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.import_buffer).desired_rows(16));
            });
        ui.horizontal(|ui| {
            if ui.button("Ok").clicked() {
                if !self.import_buffer.trim().is_empty() {
                    self.load_journey_string(self.import_buffer.clone());
                    self.import_buffer.clear();
                    self.mode = Mode::NORMAL;
                }
            }
            if ui.button("Cancel").clicked() {
                self.mode = Mode::NORMAL;
                self.import_buffer.clear();
            }
        });
    }

    fn export_panel(&mut self, ui: &mut egui::Ui) {
        let mut export_string = journey::export(&self.name, &self.gpx);
        ui.label("Exporting:");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut export_string).desired_rows(16));
            });
        ui.horizontal(|ui| {
            if ui.button("Ok").clicked() {
                if !export_string.trim().is_empty() {
                    println!("{}\n", export_string.clone());
                    if set_clipboard(export_string) {
                        self.status_text = String::from("Copied to clipboard");
                    }
                    self.mode = Mode::NORMAL;
                }
            }
            if ui.button("Cancel").clicked() {
                self.mode = Mode::NORMAL;
            }
        });
    }

    fn map_panel(&mut self, ui: &mut egui::Ui) {
        let track_color = Color32::from_rgb(66, 133, 244); // blue
        let available_height = ui.available_size().y;
        let map_height = (available_height - 60.0).max(200.0);

        let mut plot = Plot::new("track_map")
            .height(map_height)
            .data_aspect(1.0)
            .x_axis_label("Longitude")
            .y_axis_label("Latitude")
            .show_axes(true)
            .show_grid(true);

        if self.reset_plot {
            plot = plot.reset();
            self.reset_plot = false;
        }

        plot.show(ui, |plot_ui| {
            for (ti, trk) in self.gpx.tracks.iter().enumerate() {
                for seg in &trk.segments {
                    let pts: PlotPoints = seg
                        .points
                        .iter()
                        .map(|p| [p.point().x(), p.point().y()])
                        .collect();
                    let name = trk
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Track {}", ti + 1));
                    plot_ui.line(
                        Line::new("Track", pts)
                            .name(&name)
                            .color(track_color)
                            .width(2.5),
                    );
                }
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Reset")
                            .size(14.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(egui::Vec2::new(100.0, 40.0))
                    .fill(Color32::from_rgb(100, 100, 120)),
                )
                .clicked()
            {
                self.reset_plot = true;
            }
        });
    }

    fn elevation_panel(&mut self, ui: &mut egui::Ui) {
        let track_color = Color32::from_rgb(66, 244, 133); // green
        let available_height = ui.available_size().y;
        let plot_height = (available_height - 60.0).max(200.0);

        let mut plot = Plot::new("elevation_map")
            .height(plot_height)
            .x_axis_label("Distance (mi)")
            .y_axis_label("Feet")
            .show_axes(true)
            .show_grid(true);

        if self.reset_plot {
            plot = plot.reset();
            self.reset_plot = false;
        }

        plot.show(ui, |plot_ui| {
            let mut distance = 0.0;
            let mut prev: Option<(f64, f64)> = None; // (lat, lon)

            for (ti, trk) in self.gpx.tracks.iter().enumerate() {
                for seg in &trk.segments {
                    let mut pts_vec: Vec<[f64; 2]> = Vec::new();
                    for p in &seg.points {
                        let lat = p.point().y();
                        let lon = p.point().x();
                        if let Some((prev_lat, prev_lon)) = prev {
                            distance += haversine_distance(prev_lat, prev_lon, lat, lon);
                        }
                        prev = Some((lat, lon));
                        let x = km_to_mi(distance);
                        let y = m_to_ft(p.elevation.unwrap_or(0.0));
                        pts_vec.push([x, y]);
                    }

                    let pts: PlotPoints = pts_vec.into_iter().collect();
                    let name = trk
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Track {}", ti + 1));
                    plot_ui.line(
                        Line::new("Track", pts)
                            .name(&name)
                            .color(track_color)
                            .width(2.5),
                    );
                }
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Reset")
                            .size(14.0)
                            .color(Color32::WHITE),
                    )
                    .min_size(egui::Vec2::new(100.0, 40.0))
                    .fill(Color32::from_rgb(100, 100, 120)),
                )
                .clicked()
            {
                self.reset_plot = true;
            }
        });
    }

    fn load_file(&mut self, file_path: String) {
        match journey::load_gpx_file(&file_path) {
            Ok(gpx) => {
                self.gpx = gpx;
                self.distance = km_to_mi(distance(&self.gpx));
                self.min_elevation = m_to_ft(min_elevation(&self.gpx));
                self.max_elevation = m_to_ft(max_elevation(&self.gpx));
                self.diff_elevation = self.max_elevation - self.min_elevation;
                self.name = journey::name_from_gpx(&self.gpx);
                self.status_text = format!("Loaded {}", file_path);
                self.mode = Mode::NORMAL;
                self.show_map = true;
            }
            Err(e) => {
                self.status_text = format!("Failed to load GPX file: {}", e);
            }
        }
    }

    fn load_journey_string(&mut self, journey_string: String) {
        match journey::import(&journey_string) {
            Ok((name, gpx)) => {
                self.gpx = gpx;
                self.name = name.clone();
                self.distance = km_to_mi(distance(&self.gpx));
                self.min_elevation = m_to_ft(min_elevation(&self.gpx));
                self.max_elevation = m_to_ft(max_elevation(&self.gpx));
                self.diff_elevation = self.max_elevation - self.min_elevation;
                self.status_text = format!("Loaded {}", name);
            }
            Err(_) => {
                self.status_text = String::from("Failed to decode journey");
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped_files {
            if let Some(path) = file.path {
                // Only process .gpx files
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "gpx" {
                        self.load_file(path.to_string_lossy().to_string());
                    } else {
                        self.status_text = format!(
                            "Invalid file type: {}. Please drop a .gpx file.",
                            ext.to_string_lossy()
                        );
                    }
                } else {
                    self.status_text =
                        "File has no extension. Please drop a .gpx file.".to_string();
                }
            }
        }
    }
}

impl eframe::App for App {
    // called automatically every frame
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(35, 35, 45))
            .stroke(egui::Stroke::new(1.5, Color32::from_rgb(80, 80, 100)))
            .inner_margin(egui::Margin::symmetric(10, 8));

        Panel::top("top_panel").frame(frame).show_inside(ui, |ui| {
            self.top_panel(ui);
        });

        Panel::bottom("bottom_panel")
            .frame(frame)
            .show_inside(ui, |ui| {
                self.bottom_panel(ui);
            });

        CentralPanel::default().show_inside(ui, |ui| match self.mode {
            Mode::NORMAL => {
                if self.show_map {
                    self.map_panel(ui);
                } else {
                    self.elevation_panel(ui);
                }
                let ctx = ui.ctx().clone();
                self.handle_dropped_files(&ctx);
            }
            Mode::LOAD => self.load_panel(ui),
            Mode::IMPORT => self.import_panel(ui),
            Mode::EXPORT => self.export_panel(ui),
        });
    }
}

fn min_elevation(gpx: &Gpx) -> f64 {
    let mut min = f64::MAX;
    for track in &gpx.tracks {
        for segment in &track.segments {
            for waypoint in &segment.points {
                if let Some(elevation) = waypoint.elevation {
                    if elevation < min {
                        min = elevation;
                    }
                }
            }
        }
    }
    min
}

fn max_elevation(gpx: &Gpx) -> f64 {
    let mut max = f64::MIN;
    for track in &gpx.tracks {
        for segment in &track.segments {
            for waypoint in &segment.points {
                if let Some(elevation) = waypoint.elevation {
                    if elevation > max {
                        max = elevation;
                    }
                }
            }
        }
    }
    max
}

fn distance(gpx: &Gpx) -> f64 {
    let mut total = 0.0;
    for track in &gpx.tracks {
        for segment in &track.segments {
            for i in 1..segment.points.len() {
                let point = &segment.points[i];
                let prev_point = &segment.points[i - 1];
                let distance = haversine_distance(
                    prev_point.point().y(),
                    prev_point.point().x(),
                    point.point().y(),
                    point.point().x(),
                );
                total += distance;
            }
        }
    }
    total
}

fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // Earth radius in km
    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();
    let d_lat = lat2 - lat1;
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

fn km_to_mi(value: f64) -> f64 {
    value * 0.621371
}

fn m_to_ft(value: f64) -> f64 {
    value * 3.28084
}

fn read_clipboard() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => return Some(text),
                Err(_) => return None,
            },
            Err(_) => return None,
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        return None;
    }
}

#[allow(unused_variables)]
fn set_clipboard(text: String) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            match clipboard.set_text(text) {
                Ok(_) => return true,
                Err(_) => return false,
            }
        } else {
            return false;
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        return false;
    }
}
