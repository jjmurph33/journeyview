use crate::journey::{
    self, JourneySegment, distance, elevation_segments, km_to_mi, m_to_ft, max_elevation,
    min_elevation, plot_segments,
};
use eframe::egui;
use egui::{
    Align, Button, CentralPanel, Color32, ColorImage, FontFamily, FontId, Frame, Image, Key, Label,
    Layout, Margin, Panel, RichText, ScrollArea, Sense, Stroke, TextEdit, TextStyle, TextureHandle,
    TextureOptions, Ui, Vec2,
};
use egui_plot::{Line, Plot};
use gpx::Gpx;
use qrcode::QrCode;

static BUTTON_TEXT_SIZE: f32 = 22.0;

#[derive(PartialEq)]
enum Mode {
    Normal,
    Import,
    Export,
    Info,
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
    import_buffer: String,
    plot_segments: Vec<JourneySegment>,
    elevation_segments: Vec<JourneySegment>,
    mode: Mode,
    export_string: String,
    qrcode: Option<TextureHandle>,
    url: Option<String>,
    show_elevation: bool,
    reset_plot: bool,
}

impl App {
    pub fn new(gpx: Gpx, name: String, url: Option<String>) -> Self {
        let distance = km_to_mi(distance(&gpx));
        let min_elevation = m_to_ft(min_elevation(&gpx));
        let max_elevation = m_to_ft(max_elevation(&gpx));
        let diff_elevation = max_elevation - min_elevation;
        let plot_segments = plot_segments(&gpx);
        let elevation_segments = elevation_segments(&gpx);
        Self {
            gpx,
            name,
            distance,
            min_elevation,
            max_elevation,
            diff_elevation,
            name_editing: false,
            name_buffer: String::new(),
            import_buffer: String::new(),
            plot_segments,
            elevation_segments,
            mode: Mode::Normal,
            export_string: String::new(),
            qrcode: None,
            url,
            show_elevation: false,
            reset_plot: true,
        }
    }

    fn load(&mut self, gpx: Gpx, name: Option<String>) {
        self.gpx = gpx;
        self.distance = km_to_mi(distance(&self.gpx));
        self.min_elevation = m_to_ft(min_elevation(&self.gpx));
        self.max_elevation = m_to_ft(max_elevation(&self.gpx));
        self.diff_elevation = self.max_elevation - self.min_elevation;
        self.plot_segments = plot_segments(&self.gpx);
        self.elevation_segments = elevation_segments(&self.gpx);
        self.name = if let Some(name) = name {
            name
        } else {
            journey::name_from_gpx(&self.gpx)
        };
        self.reset_ui();
    }

    fn reset_ui(&mut self) {
        self.mode = Mode::Normal;
        self.show_elevation = false;
        self.name_editing = false;
        self.name_buffer.clear();
        self.import_buffer.clear();
        self.reset_plot = true;
        self.export_string.clear();
        self.qrcode = None;
    }

    fn top_panel(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                //////////// Name label ////////////////////////
                ui.horizontal(|ui| {
                    if self.name_editing {
                        if self.name_buffer.is_empty() {
                            self.name_buffer = self.name.clone();
                        }
                        let name_edit_id = egui::Id::new("name_edit"); // need id to manage focus
                        ui.add(
                            TextEdit::singleline(&mut self.name_buffer)
                                .id(name_edit_id)
                                .font(TextStyle::Heading),
                        );
                        if !ui.memory(|m| m.has_focus(name_edit_id)) {
                            ui.memory_mut(|m| m.request_focus(name_edit_id));
                        }

                        if ui.small_button("Rename").clicked()
                            || ui.ctx().input(|i| i.key_pressed(Key::Enter))
                        {
                            self.name = self.name_buffer.clone();
                            self.name_editing = false;
                        }
                        if ui.small_button("Cancel").clicked()
                            || ui.ctx().input(|i| i.key_pressed(Key::Escape))
                        {
                            self.name_buffer.clear();
                            self.name_editing = false;
                        }
                    } else {
                        let label_button = ui.add(
                            Label::new(
                                RichText::new(self.name.clone())
                                    .size(28.0)
                                    .color(Color32::from_rgb(200, 220, 255))
                                    .strong(),
                            )
                            .sense(Sense::click()),
                        );
                        if label_button.clicked() {
                            self.name_editing = true;
                            self.name_buffer = self.name.clone();
                        }
                    }
                });
                ///////////////////// Info labels /////////////////////
                ui.label(
                    RichText::new(format!("Distance: {:.1}mi", self.distance))
                        .size(18.0)
                        .color(Color32::from_rgb(210, 210, 220)),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Elevation: {:.0}ft -> {:.0}ft ({:.0}ft change)",
                            self.min_elevation, self.max_elevation, self.diff_elevation
                        ))
                        .size(18.0)
                        .color(Color32::from_rgb(210, 210, 220)),
                    );
                    let button = ui.add(
                        Button::new(
                            RichText::new("\u{2139}")
                                .size(10.0)
                                .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .fill(Color32::from_rgb(33, 150, 243)) // Blue
                        .corner_radius(5.0),
                    );
                    if button.clicked() {
                        if self.mode == Mode::Info {
                            self.mode = Mode::Normal;
                        } else {
                            self.mode = Mode::Info;
                        }
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ///////////////////// Export button ////////////////////////
                if ui
                    .add(
                        Button::new(
                            RichText::new("Export")
                                .size(BUTTON_TEXT_SIZE)
                                .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .min_size(Vec2::new(150.0, 50.0))
                        .fill(Color32::from_rgb(33, 150, 243)) // Blue
                        .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                    )
                    .clicked()
                {
                    self.mode = Mode::Export;
                }
                ///////////////////// Import button ////////////////////////
                if ui
                    .add(
                        Button::new(
                            RichText::new("Import")
                                .size(BUTTON_TEXT_SIZE)
                                .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .min_size(Vec2::new(150.0, 50.0))
                        .fill(Color32::from_rgb(33, 150, 243)) // Blue
                        .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                    )
                    .clicked()
                {
                    self.mode = Mode::Import;
                    if let Some(clipboard_text) = read_clipboard() {
                        self.import_buffer = clipboard_text;
                    } else {
                        self.import_buffer.clear();
                    }
                }
                ///////////////////// Load button ////////////////////////
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.add_space(12.0);
                    if ui
                        .add(
                            Button::new(
                                RichText::new("Load File")
                                    .size(BUTTON_TEXT_SIZE)
                                    .color(Color32::from_rgb(255, 255, 255)),
                            )
                            .min_size(Vec2::new(150.0, 50.0))
                            .fill(Color32::from_rgb(33, 150, 243)) // Blue
                            .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                        )
                        .clicked()
                    {
                        self.open_gpx_picker();
                    }
                }
                ///////////////////// Elevation/Map button //////////////////
                ui.add_space(28.0);
                if ui
                    .add(
                        Button::new(
                            RichText::new(if self.show_elevation {
                                "Map"
                            } else {
                                "Elevation"
                            })
                            .size(BUTTON_TEXT_SIZE)
                            .color(Color32::from_rgb(255, 255, 255)),
                        )
                        .min_size(Vec2::new(150.0, 50.0))
                        .fill(Color32::from_rgb(76, 175, 80)) // Green
                        .stroke(Stroke::new(1.5, Color32::from_rgb(56, 142, 60))),
                    )
                    .clicked()
                {
                    self.show_elevation = !self.show_elevation;
                }
            });
        });
    }

    fn import_panel(&mut self, ui: &mut Ui) {
        ui.label("Journey Code:");
        ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                TextEdit::multiline(&mut self.import_buffer)
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .hint_text(String::from("paste here")),
            );
        });
        ui.horizontal(|ui| {
            if ui
                .add(
                    Button::new(
                        RichText::new("Import")
                            .size(16.0)
                            .color(Color32::from_rgb(255, 255, 255)),
                    )
                    .min_size(Vec2::new(150.0, 50.0))
                    .fill(Color32::from_rgb(33, 150, 243)) // Blue
                    .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                )
                .clicked()
                && !self.import_buffer.trim().is_empty()
            {
                self.load_journey_string(self.import_buffer.clone());
                self.import_buffer.clear();
                self.mode = Mode::Normal;
            }
            if ui
                .add(
                    Button::new(
                        RichText::new("Cancel")
                            .size(16.0)
                            .color(Color32::from_rgb(255, 255, 255)),
                    )
                    .min_size(Vec2::new(150.0, 50.0))
                    .fill(Color32::from_rgb(33, 150, 243)) // Blue
                    .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                )
                .clicked()
            {
                self.mode = Mode::Normal;
                self.import_buffer.clear();
            }
        });
    }

    fn export_panel(&mut self, ui: &mut Ui) {
        if self.export_string.is_empty() {
            self.export_string = journey::export(&self.name, &self.gpx);
            let include_url = true; //TODO: make this a radio button
            if include_url
                && !self.export_string.is_empty()
                && let Some(url) = self.url.as_ref()
            {
                let export_url = format!("{}/?j=", url);
                self.export_string.insert_str(0, &export_url);
            }
        }

        if self.qrcode.is_none() && !self.export_string.is_empty() {
            self.qrcode = Some(qr_to_texture(ui.ctx(), &self.export_string));
        }

        let id = egui::Id::new("export_text");
        let initialized = ui.memory(|m| m.data.get_temp::<bool>(id)).unwrap_or(false);

        let mut size = ui.available_size();
        size.x /= 2.0;
        size.y -= 80.0;

        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            ui.add_sized(
                size,
                TextEdit::multiline(&mut self.export_string.as_str())
                    .font(FontId::new(18.0, FontFamily::Proportional))
                    .desired_width(f32::INFINITY)
                    .desired_rows(20)
                    .id(id),
            );
            // select all of the text
            if !initialized {
                let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.export_string.clone().chars().count()),
                    )));
                state.store(ui.ctx(), id);
                ui.memory_mut(|m| {
                    m.request_focus(id);
                    // set initialized flag
                    m.data.insert_temp(id, true);
                });
            }

            if let Some(texture) = &self.qrcode {
                ui.add_sized(size, Image::new(texture).shrink_to_fit());
            } else {
                ui.add_sized(size, Label::new("Error Generating QRCode"));
            }
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    Button::new(
                        RichText::new("Cancel")
                            .size(16.0)
                            .color(Color32::from_rgb(255, 255, 255)),
                    )
                    .min_size(Vec2::new(150.0, 50.0))
                    .fill(Color32::from_rgb(33, 150, 243)) // Blue
                    .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                )
                .clicked()
            {
                self.mode = Mode::Normal;
            }
            if ui
                .add(
                    Button::new(
                        RichText::new("Copy to clipboard")
                            .size(16.0)
                            .color(Color32::from_rgb(255, 255, 255)),
                    )
                    .min_size(Vec2::new(150.0, 50.0))
                    .fill(Color32::from_rgb(33, 150, 243)) // Blue
                    .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                )
                .clicked()
                && !self.export_string.trim().is_empty()
            {
                println!("{}\n", self.export_string.clone());
                if set_clipboard(self.export_string.clone()) {
                    // TODO: show a status message that the text was copied to the clipboard
                }
                self.mode = Mode::Normal;
            }
        });
    }

    fn info_panel(&mut self, ui: &mut Ui) {
        ui.label("GPX Info");
        ScrollArea::vertical().show(ui, |ui| {
            ui.label(journey::info(&self.gpx));
        });

        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            if ui
                .add(
                    Button::new(
                        RichText::new("Ok")
                            .size(16.0)
                            .color(Color32::from_rgb(255, 255, 255)),
                    )
                    .min_size(Vec2::new(150.0, 50.0))
                    .fill(Color32::from_rgb(33, 150, 243)) // Blue
                    .stroke(Stroke::new(1.5, Color32::from_rgb(21, 101, 192))),
                )
                .clicked()
            {
                self.mode = Mode::Normal;
            }
        });
    }

    fn map_panel(&mut self, ui: &mut Ui, elevation: bool) {
        let available_height = ui.available_size().y;

        let mut plot = if elevation {
            Plot::new("elevation_plot")
                .x_axis_label("Distance (mi)")
                .y_axis_label("Feet")
        } else {
            Plot::new("map_plot")
                .x_axis_label("Longitude")
                .y_axis_label("Latitude")
        };

        plot = plot
            .height(available_height)
            .show_axes(true)
            .show_grid(true)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false);

        if self.reset_plot {
            plot = plot.reset();
            self.reset_plot = false;
        }

        // draw the plot and save the response
        let response = if elevation {
            plot.show(ui, |plot_ui| {
                for segment in &self.elevation_segments {
                    plot_ui.line(
                        Line::new("Track", segment.points.clone())
                            .name(&segment.name)
                            .color(segment.color)
                            .width(2.5),
                    );
                }
                let scroll_delta = plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 {
                    let zoom_factor = if scroll_delta > 0.0 { 1.0 / 1.1 } else { 1.1 };
                    plot_ui.zoom_bounds_around_hovered(Vec2::splat(zoom_factor));
                }
            })
        } else {
            plot.show(ui, |plot_ui| {
                for segment in &self.plot_segments {
                    plot_ui.line(
                        Line::new("Track", segment.points.clone())
                            .name(&segment.name)
                            .color(segment.color)
                            .width(2.5),
                    );
                }
                let scroll_delta = plot_ui.ctx().input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 {
                    let zoom_factor = if scroll_delta > 0.0 { 1.0 / 1.1 } else { 1.1 };
                    plot_ui.zoom_bounds_around_hovered(Vec2::splat(zoom_factor));
                }
            })
        };

        // draw the reset button on top of the plot
        let plot_rect = response.response.rect;
        let btn_size = Vec2::new(56.0, 36.0);
        let padding = Vec2::new(12.0, 12.0);
        let btn_min = egui::pos2(
            plot_rect.right() - padding.x - btn_size.x,
            plot_rect.bottom() - padding.y - btn_size.y,
        );
        let btn_rect = egui::Rect::from_min_size(btn_min, btn_size);
        let resp = ui
            .put(
                btn_rect,
                Button::new(RichText::new("\u{1F504}").size(20.0))
                    .fill(Color32::from_rgb(33, 150, 243))
                    .stroke(Stroke::new(1.5, Color32::BLACK)),
            )
            .on_hover_text("Reset");
        if resp.clicked() {
            self.reset_plot = true;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_gpx_picker(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("GPX Files", &["gpx"])
            .set_title("Open GPX File")
            .pick_file();

        if let Some(path) = path {
            self.load_file(path.display().to_string());
        } else {
            // TODO: show a status message that the user canceled the file picker
        }
    }

    fn load_file(&mut self, file_path: String) {
        match journey::load_gpx_file(&file_path) {
            Ok(gpx) => {
                self.load(gpx, None);
            }
            Err(_e) => {
                // TODO: show a status message that the file could not be loaded
            }
        }
    }

    fn load_journey_string(&mut self, journey_string: String) {
        match journey::import(&journey_string) {
            Ok((name, gpx)) => {
                self.load(gpx, Some(name.clone()));
            }
            Err(_) => {
                // TODO: show a status message that the string could not be imported
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped_files {
            let path = file.path();
            // Only process .gpx files
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "gpx" {
                    self.load_file(path.to_string_lossy().to_string());
                } else {
                    // TODO: show a status message that the file type is not supported
                }
            } else {
                // TODO: show a status message that the file has no extension
            }
        }
    }
}

impl eframe::App for App {
    // called automatically every frame
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let frame = Frame::default()
            .fill(Color32::from_rgb(35, 35, 45))
            .stroke(Stroke::new(1.5, Color32::from_rgb(80, 80, 100)))
            .inner_margin(Margin::symmetric(10, 8));

        Panel::top("top_panel").frame(frame).show(ui, |ui| {
            self.top_panel(ui);
        });

        CentralPanel::default().show(ui, |ui| match self.mode {
            Mode::Normal => {
                if self.show_elevation {
                    self.map_panel(ui, true);
                } else {
                    self.map_panel(ui, false);
                }
                // TODO: why only in Normal mode?
                let ctx = ui.ctx().clone();
                self.handle_dropped_files(&ctx);
            }
            Mode::Import => self.import_panel(ui),
            Mode::Export => self.export_panel(ui),
            Mode::Info => self.info_panel(ui),
        });
    }
}

fn read_clipboard() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => clipboard.get_text().ok(),
            Err(_) => None,
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
            clipboard.set_text(text).is_ok()
        } else {
            false
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        return false;
    }
}

fn qr_to_texture(ctx: &egui::Context, data: &str) -> TextureHandle {
    //println!("length = {}", data.len());
    //println!("{}", &data);

    let code = QrCode::new(data).unwrap();
    //let code = QrCode::with_error_correction_level(data,qrcode::EcLevel::L).unwrap();

    // Get the raw bool matrix
    let bits: Vec<Vec<bool>> = code
        .to_colors()
        .chunks(code.width())
        .map(|row| row.iter().map(|c| *c == qrcode::Color::Dark).collect())
        .collect();

    let scale = 8usize; // pixels per module
    let size = bits.len() * scale;

    let mut pixels = vec![egui::Color32::WHITE; size * size];

    for (y, row) in bits.iter().enumerate() {
        for (x, &dark) in row.iter().enumerate() {
            let color = if dark {
                egui::Color32::BLACK
            } else {
                egui::Color32::WHITE
            };
            for dy in 0..scale {
                for dx in 0..scale {
                    pixels[(y * scale + dy) * size + (x * scale + dx)] = color;
                }
            }
        }
    }

    let image = ColorImage::from_rgba_unmultiplied(
        [size, size],
        &pixels
            .iter()
            .flat_map(|c| c.to_array())
            .collect::<Vec<u8>>(),
    );

    ctx.load_texture("qrcode", image, TextureOptions::NEAREST)
}
