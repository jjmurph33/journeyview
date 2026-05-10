use eframe::egui;
use egui::{CentralPanel, Color32, Panel, Ui};
use egui_plot::{Line, Plot, PlotPoints};
use gpx::Gpx;

#[derive(Default)]
pub struct App {
    gpx_data: Gpx,
    name: String,
    distance: f64,       // miles
    min_elevation: f64,  // feet
    max_elevation: f64,  // feet
    diff_elevation: f64, // feet
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, gpx_data: Gpx, name: String) -> Self {
        let distance = km_to_mi(distance(&gpx_data));
        let min_elevation = m_to_ft(min_elevation(&gpx_data));
        let max_elevation = m_to_ft(max_elevation(&gpx_data));
        let diff_elevation = max_elevation - min_elevation;
        Self {
            gpx_data,
            name,
            distance,
            min_elevation,
            max_elevation,
            diff_elevation,
        }
    }

    fn top_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.name);
            ui.separator();
            ui.label(format!("Distance: {:.1}mi", self.distance));
            ui.separator();
            ui.label(format!(
                "Elevation: {:.0}ft -> {:.0}ft ({:.0}ft)",
                self.min_elevation, self.max_elevation, self.diff_elevation
            ))
        });
    }

    fn map_panel(&mut self, ui: &mut egui::Ui) {
        let track_color = Color32::from_rgb(66, 133, 244); // blue

        let available_height = ui.available_size().y;
        let map_height = (available_height / 2.0).max(200.0);

        let plot = Plot::new("track_map")
            .height(map_height)
            .data_aspect(1.0)
            .x_axis_label("Longitude")
            .y_axis_label("Latitude")
            .show_axes(true)
            .show_grid(true);

        plot.show(ui, |plot_ui| {
            for (ti, trk) in self.gpx_data.tracks.iter().enumerate() {
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
    }

    fn elevation_panel(&mut self, ui: &mut egui::Ui) {
        let track_color = Color32::from_rgb(66, 244, 133); // green

        let plot = Plot::new("elevation_map")
            .x_axis_label("Distance (mi)")
            .y_axis_label("Feet")
            .show_axes(true)
            .show_grid(true);

        plot.show(ui, |plot_ui| {
            let mut distance = 0.0;
            let mut prev: Option<(f64, f64)> = None; // (lat, lon)

            for (ti, trk) in self.gpx_data.tracks.iter().enumerate() {
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
    }
}

impl eframe::App for App {
    // called automatically every frame
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        Panel::top("top_panel").show_inside(ui, |ui| {
            self.top_panel(ui);
        });

        CentralPanel::default().show_inside(ui, |ui| {
            self.map_panel(ui);
            ui.separator();
            self.elevation_panel(ui);
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
