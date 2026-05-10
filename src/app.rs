use eframe::egui;
use egui::{CentralPanel, Color32, Panel, Ui};
use egui_plot::{Line, Plot, PlotPoints};
use gpx::Gpx;

#[derive(Default)]
pub struct App {
    gpx_data: Gpx,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, gpx_data: Gpx) -> Self {
        Self {
            gpx_data,
            ..Default::default()
        }
    }

    fn top_panel(&mut self, ui: &mut egui::Ui) {
        let mut output_name = &"GPX Viewer".to_string();
        if let Some(metadata) = &self.gpx_data.metadata {
            if let Some(name) = &metadata.name {
                output_name = &name;
            }
        }

        ui.horizontal(|ui| {
            ui.label(output_name);
            ui.separator();

            ui.label(format!("Distance: {:.1}mi", km_to_mi(self.distance())));
            ui.separator();

            let min_elevation = mi_to_ft(km_to_mi(self.min_elevation()));
            let max_elevation = mi_to_ft(km_to_mi(self.max_elevation()));
            let diff_elevation = max_elevation - min_elevation;
            ui.label(format!(
                "Elevation: {:.0}ft -> {:.0}ft ({:.0}ft)",
                min_elevation, max_elevation, diff_elevation
            ))
        });
    }

    fn map_panel(&mut self, ui: &mut egui::Ui) {
        let track_color = Color32::from_rgb(66, 133, 244); // blue

        let plot = Plot::new("track_map")
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
        ui.label("elevation");
        for track in &self.gpx_data.tracks {
            for segment in &track.segments {
                for waypoint in &segment.points {
                    let elevation = waypoint.elevation.unwrap_or(0.0);
                    ui.label(format!("{}", elevation));
                }
            }
        }
    }

    fn min_elevation(&self) -> f64 {
        let mut min = f64::MAX;
        for track in &self.gpx_data.tracks {
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

    fn max_elevation(&self) -> f64 {
        let mut max = f64::MIN;
        for track in &self.gpx_data.tracks {
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

    fn distance(&self) -> f64 {
        let mut total = 0.0;
        for track in &self.gpx_data.tracks {
            for segment in &track.segments {
                for i in 1..segment.points.len() {
                    let point = &segment.points[i];
                    let prev_point = &segment.points[i - 1];
                    let distance = haversine_distance(
                        prev_point.point().x(),
                        prev_point.point().y(),
                        point.point().x(),
                        point.point().y(),
                    );
                    total += distance;
                }
            }
        }
        total
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

fn mi_to_ft(value: f64) -> f64 {
    value * 3.28084
}
