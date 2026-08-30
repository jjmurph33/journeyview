use crate::journey::{
    self, JourneySegment, distance, elevation_segments, km_to_mi, m_to_ft, max_elevation,
    min_elevation, plot_segments,
};
use eframe::egui;
use egui::{
    Align, Button, CentralPanel, CollapsingHeader, Color32, ColorImage, Context, FontFamily,
    FontId, Frame, Image, Key, Label, Layout, Margin, Panel, RichText, ScrollArea, Stroke,
    TextEdit, TextureHandle, TextureOptions, Ui, Vec2,
};
use egui_plot::{Line, Plot};
use gpx::Gpx;
use poll_promise::Promise;
use qrcode::QrCode;
use rfd::AsyncFileDialog;
use walkers::{
    HttpTiles, Map, MapMemory, Plugin, Position, Projector, lon_lat, sources::OpenStreetMap,
};

static BUTTON_TEXT_SIZE: f32 = 22.0;
static BUTTON_HEIGHT: f32 = 50.0;
static COMPACT_WIDTH: f32 = 900.0;

#[derive(PartialEq, Default)]
enum Mode {
    #[default]
    Normal,
    Import,
    Export,
    Info,
}

#[derive(Default)]
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
    plot_segments: Option<Vec<JourneySegment>>,
    elevation_segments: Option<Vec<JourneySegment>>,
    mode: Mode,
    export_string: String,
    qrcode: Option<TextureHandle>,
    url: Option<String>,
    show_elevation: bool,
    reset_plot: bool,
    include_url: bool,
    map_tiles: Option<HttpTiles>,
    map_memory: MapMemory,
    map_center: Position,
    file_promise: Option<Promise<Result<Option<Vec<u8>>, String>>>,
    error_message: Option<String>,
}

impl App {
    pub fn new(ctx: &Context, gpx: Gpx, name: String, url: Option<String>) -> Self {
        let mut app = Self {
            gpx,
            name,
            url,
            ..Default::default()
        };
        app.init(ctx);
        app
    }

    fn init(&mut self, ctx: &Context) {
        self.distance = km_to_mi(distance(&self.gpx));
        self.min_elevation = m_to_ft(min_elevation(&self.gpx));
        self.max_elevation = m_to_ft(max_elevation(&self.gpx));
        self.diff_elevation = self.max_elevation - self.min_elevation;
        self.plot_segments = Some(plot_segments(&self.gpx));
        self.elevation_segments = Some(elevation_segments(&self.gpx));
        self.include_url = self.url.is_some();
        self.map_tiles = Some(openstreetmap_tiles(ctx));
        self.map_center = route_center(self.plot_segments.as_deref().unwrap_or_default());
        self.map_memory = MapMemory::default();
        self.map_memory.center_at(self.map_center);
    }

    fn load(&mut self, ctx: &Context, gpx: Gpx, name: Option<String>) {
        self.error_message = None;
        self.gpx = gpx;
        self.name = if let Some(name) = name {
            name
        } else {
            journey::name_from_gpx(&self.gpx)
        };
        self.init(ctx);
        self.reset_ui();

        log::info!("Loaded journey '{}'", self.name);
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
        let compact = ui.available_width() < COMPACT_WIDTH;

        self.journey_summary(ui, compact);
        ui.add_space(8.0);
        self.top_actions(ui, compact);

        if let Some(error) = &self.error_message {
            ui.add_space(4.0);
            ui.label(RichText::new(error).color(Color32::from_rgb(255, 120, 120)));
        }
    }

    fn journey_summary(&mut self, ui: &mut Ui, compact: bool) {
        let save = self.name_editing && (ui.ctx().input(|i| i.key_pressed(Key::Enter)));
        let cancel = self.name_editing && (ui.ctx().input(|i| i.key_pressed(Key::Escape)));

        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            let mut save = save;
            let mut cancel = cancel;
            if self.name_editing {
                cancel |= ui.small_button("Cancel").clicked();
                save |= ui.small_button("Save").clicked();
            } else if ui.small_button("Rename").clicked() {
                self.name_editing = true;
                self.name_buffer = self.name.clone();
            }

            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 0.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    if self.name_editing {
                        if self.name_buffer.is_empty() {
                            self.name_buffer = self.name.clone();
                        }
                        let name_edit_id = egui::Id::new("name_edit");
                        let edit = TextEdit::singleline(&mut self.name_buffer)
                            .id(name_edit_id)
                            .font(if compact {
                                FontId::new(16.0, FontFamily::Proportional)
                            } else {
                                FontId::new(22.0, FontFamily::Proportional)
                            });
                        ui.add_sized(Vec2::new(ui.available_width(), 30.0), edit);
                        if !ui.memory(|m| m.has_focus(name_edit_id)) {
                            ui.memory_mut(|m| m.request_focus(name_edit_id));
                        }
                    } else {
                        ui.add(
                            Label::new(
                                RichText::new(self.name.clone())
                                    .size(if compact { 22.0 } else { 28.0 })
                                    .color(Color32::from_rgb(200, 220, 255))
                                    .strong(),
                            )
                            .wrap(),
                        );
                    }
                },
            );

            if save {
                self.name = self.name_buffer.clone();
                self.export_string.clear();
                self.name_editing = false;
            } else if cancel {
                self.name_buffer.clear();
                self.name_editing = false;
            }
        });

        let stat_size = if compact { 15.0 } else { 18.0 };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Distance: {:.1}mi", self.distance))
                    .size(stat_size)
                    .color(Color32::from_rgb(210, 210, 220)),
            );
            let button = ui.add(
                Button::new(RichText::new("\u{2139}").size(10.0).color(Color32::WHITE))
                    .fill(Color32::from_rgb(33, 150, 243))
                    .corner_radius(5.0),
            );
            if button.clicked() {
                self.mode = if self.mode == Mode::Info {
                    Mode::Normal
                } else {
                    Mode::Info
                };
            }
        });
    }

    fn top_actions(&mut self, ui: &mut Ui, compact: bool) {
        let spacing = ui.spacing().item_spacing.x * 3.0;
        let button_width = if compact {
            ((ui.available_width() - spacing) / 4.0).max(0.0)
        } else {
            150.0
        };
        let button_height = if compact { 44.0 } else { 50.0 };
        let text_size = if compact && button_width < 90.0 {
            12.0
        } else if compact {
            14.0
        } else {
            BUTTON_TEXT_SIZE
        };

        let render_buttons = |ui: &mut Ui, app: &mut Self, order: [u8; 4]| {
            for action in order {
                match action {
                    0 => {
                        if top_action_button(
                            ui,
                            "Export",
                            button_width,
                            button_height,
                            text_size,
                            Color32::from_rgb(33, 150, 243),
                            Color32::from_rgb(21, 101, 192),
                        ) {
                            app.mode = Mode::Export;
                        }
                    }
                    1 => {
                        if top_action_button(
                            ui,
                            "Import",
                            button_width,
                            button_height,
                            text_size,
                            Color32::from_rgb(33, 150, 243),
                            Color32::from_rgb(21, 101, 192),
                        ) {
                            app.mode = Mode::Import;
                            if let Some(clipboard_text) = read_clipboard() {
                                app.import_buffer = clipboard_text;
                            } else {
                                app.import_buffer.clear();
                            }
                        }
                    }
                    2 => {
                        if top_action_button(
                            ui,
                            "Load File",
                            button_width,
                            button_height,
                            text_size,
                            Color32::from_rgb(33, 150, 243),
                            Color32::from_rgb(21, 101, 192),
                        ) {
                            app.open_gpx_file();
                        }
                    }
                    3 => {
                        if top_action_button(
                            ui,
                            if app.show_elevation {
                                "Map"
                            } else {
                                "Elevation"
                            },
                            button_width,
                            button_height,
                            text_size,
                            Color32::from_rgb(76, 175, 80),
                            Color32::from_rgb(56, 142, 60),
                        ) {
                            app.show_elevation = !app.show_elevation;
                            app.mode = Mode::Normal;
                        }
                    }
                    _ => unreachable!(),
                }
            }
        };

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            render_buttons(ui, self, [3, 2, 1, 0]);
        });
    }

    fn import_panel(&mut self, ui: &mut Ui) {
        ui.label("Journey Code:");
        let scroll_height =
            ui.available_height() - BUTTON_HEIGHT - ui.spacing().item_spacing.y * 2.0;
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), scroll_height),
            Layout::left_to_right(Align::TOP),
            |ui| {
                let text_height = ui.available_height();
                ScrollArea::vertical()
                    .min_scrolled_height(text_height)
                    .max_height(text_height)
                    .show(ui, |ui| {
                        ui.add_sized(
                            Vec2::new(ui.available_width(), text_height),
                            TextEdit::multiline(&mut self.import_buffer)
                                .font(FontId::new(18.0, FontFamily::Proportional))
                                .hint_text(String::from("paste here")),
                        );
                    });
            },
        );
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
                self.load_journey_string(ui.ctx(), self.import_buffer.clone());
                self.reset_ui();
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
        let compact = ui.available_width() < COMPACT_WIDTH;
        let content_height =
            (ui.available_height() - BUTTON_HEIGHT - ui.spacing().item_spacing.y * 8.0).max(0.0);
        let horizontal_size = Vec2::new(ui.available_width() * 0.5, content_height);

        let mut url = self.url.clone().unwrap_or_default();

        if self.export_string.is_empty() {
            self.export_string = journey::export(&self.name, &self.gpx);
            log::info!(
                "Generated export payload for '{}' ({} chars)",
                self.name,
                self.export_string.len()
            );
        }
        let mut export_string = self.export_string.clone(); // temp copy that may include url

        let mut changed = false; // need to regenerate qrcode if text changes

        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            let content_layout = if compact {
                Layout::top_down(Align::LEFT)
            } else {
                Layout::left_to_right(Align::TOP)
            };

            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), content_height),
                content_layout,
                |ui| {
                    let text_size = if compact {
                        Vec2::new(ui.available_width(), content_height * 0.35)
                    } else {
                        horizontal_size
                    };

                    if self.include_url {
                        let export_url = format!("{}/?j=", url.trim_end_matches("/"));
                        export_string.insert_str(0, &export_url);
                    }

                    CollapsingHeader::new("Export Text".to_string())
                        .default_open(!compact)
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                text_size,
                                Layout::top_down(Align::LEFT),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .checkbox(&mut self.include_url, "Include URL: ")
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        if ui
                                            .add_enabled(
                                                self.include_url,
                                                TextEdit::singleline(&mut url),
                                            )
                                            .changed()
                                        {
                                            self.url.replace(url.to_string());
                                            changed = true;
                                        };
                                    });

                                    let id = egui::Id::new("export_text");
                                    let text_height = ui.available_height();
                                    let initialized =
                                        ui.memory(|m| m.data.get_temp::<bool>(id)).unwrap_or(false);

                                    ui.add_space(1.0);

                                    ScrollArea::vertical()
                                        .min_scrolled_height(text_height)
                                        .max_height(text_height)
                                        .show(ui, |ui| {
                                            ui.add_sized(
                                                Vec2::new(ui.available_width(), text_height),
                                                TextEdit::multiline(&mut export_string.as_str())
                                                    .font(FontId::new(
                                                        18.0,
                                                        FontFamily::Proportional,
                                                    ))
                                                    .id(id),
                                            );
                                        });

                                    // TODO: need a way to reset this after leaving the panel and rentering
                                    // select all of the text when first entering
                                    if !initialized {
                                        let mut state =
                                            TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                                        state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::two(
                                                egui::text::CCursor::new(0),
                                                egui::text::CCursor::new(
                                                    export_string.clone().chars().count(),
                                                ),
                                            ),
                                        ));
                                        state.store(ui.ctx(), id);
                                        ui.memory_mut(|m| {
                                            m.request_focus(id); // give focus to the text area
                                            m.data.insert_temp(id, true); // set initialized flag
                                        });
                                    }
                                },
                            );
                        });

                    if changed {
                        export_string = self.export_string.clone();
                        if self.include_url {
                            let export_url = format!("{}/?j=", url.trim_end_matches("/"));
                            export_string.insert_str(0, &export_url);
                        }
                    }

                    if self.qrcode.is_none() || changed {
                        self.qrcode = qr_to_texture(ui.ctx(), &export_string); // generate the QR code
                    }
                    if let Some(texture) = &self.qrcode {
                        if compact {
                            let qr_area = ui.available_size();
                            ui.allocate_ui_with_layout(
                                qr_area,
                                Layout::top_down(Align::Center),
                                |ui| {
                                    let side = ui.available_width().min(ui.available_height());
                                    ui.add(
                                        Image::new(texture)
                                            .fit_to_exact_size(Vec2::splat(side.max(0.0))),
                                    );
                                },
                            );
                        } else {
                            ui.add_sized(horizontal_size, Image::new(texture).shrink_to_fit());
                        }
                    } else {
                        let error_size = if compact {
                            ui.available_size()
                        } else {
                            horizontal_size
                        };
                        ui.add_sized(error_size, Label::new("Error Generating QRCode"));
                    }
                },
            );

            ui.horizontal(|ui| {
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
                    && !export_string.trim().is_empty()
                {
                    ui.copy_text(export_string.clone());
                    self.mode = Mode::Normal;
                    log::info!(
                        "Copied exported journey to clipboard ({} chars)",
                        export_string.len()
                    );
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
                }
            });
        });
    }

    fn info_panel(&mut self, ui: &mut Ui) {
        let content_height =
            ui.available_height() - BUTTON_HEIGHT - ui.spacing().item_spacing.y * 2.0;

        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), content_height),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.label("GPX Info");
                    ui.label(
                        RichText::new(format!(
                            "Elevation: {:.0}ft to {:.0}ft ({:.0}ft change)",
                            self.min_elevation, self.max_elevation, self.diff_elevation
                        ))
                        .size(18.0)
                        .color(Color32::from_rgb(210, 210, 220)),
                    );
                    ui.add_space(6.0);
                    let text_height = ui.available_height();
                    ScrollArea::vertical()
                        .min_scrolled_height(text_height)
                        .max_height(text_height)
                        .show(ui, |ui| {
                            ui.add_sized(
                                Vec2::new(ui.available_width(), text_height),
                                TextEdit::multiline(&mut journey::info(&self.gpx)),
                            );
                        });
                },
            );

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
        });
    }

    fn _map_panel_old(&mut self, ui: &mut Ui, elevation: bool) {
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

        // draw the plot (either map or elevation) and save the response
        let response = if elevation {
            plot.show(ui, |plot_ui| {
                if let Some(segments) = &self.elevation_segments {
                    for segment in segments {
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
                }
            })
        } else {
            plot.show(ui, |plot_ui| {
                if let Some(segments) = &self.plot_segments {
                    for segment in segments {
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
                Button::new(RichText::new("\u{1F504}").size(20.0)) // the reload character
                    .fill(Color32::from_rgb(33, 150, 243))
                    .stroke(Stroke::new(1.5, Color32::BLACK)),
            )
            .on_hover_text("Reset");
        if resp.clicked() {
            self.reset_plot = true;
        }
    }

    fn elevation_panel(&mut self, ui: &mut Ui) {
        let mut plot = Plot::new("elevation_plot")
            .x_axis_label("Distance (mi)")
            .y_axis_label("Feet")
            .height(ui.available_height())
            .show_axes(true)
            .show_grid(true)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false);

        if self.reset_plot {
            plot = plot.reset();
            self.reset_plot = false;
        }

        let response = plot.show(ui, |plot_ui| {
            if let Some(segments) = &self.elevation_segments {
                for segment in segments {
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
            }
        });

        if reset_button(ui, response.response.rect).clicked() {
            self.reset_plot = true;
        }
    }

    fn map_panel(&mut self, ui: &mut Ui) {
        let size = ui.available_size();
        if self.reset_plot {
            fit_map_to_segments(
                &mut self.map_memory,
                &mut self.map_center,
                self.plot_segments.as_deref().unwrap_or_default(),
                size,
            );
            self.reset_plot = false;
        }

        let Some(tiles) = self.map_tiles.as_mut() else {
            ui.centered_and_justified(|ui| ui.label("Map tiles are unavailable"));
            return;
        };

        let segments = self.plot_segments.as_deref().unwrap_or_default();
        let response = ui.add(
            Map::new(Some(tiles), &mut self.map_memory, self.map_center)
                .with_plugin(RouteLayer { segments })
                .zoom_with_ctrl(false),
        );

        let attribution_rect = egui::Rect::from_min_size(
            response.rect.left_bottom() + egui::vec2(8.0, -28.0),
            Vec2::new(205.0, 20.0),
        );
        ui.painter()
            .rect_filled(attribution_rect, 3.0, Color32::from_black_alpha(185));
        ui.put(
            attribution_rect,
            egui::Hyperlink::from_label_and_url(
                "© OpenStreetMap contributors",
                "https://www.openstreetmap.org/copyright",
            ),
        );

        if reset_button(ui, response.rect).clicked() {
            self.reset_plot = true;
        }
    }

    fn open_gpx_file(&mut self) {
        if self.file_promise.is_some() {
            self.error_message = Some("A GPX file is already loading".to_string());
            return;
        }
        self.error_message = None;
        let (sender, promise) = Promise::new();
        log::info!("Opening GPX file picker");

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let result = pollster::block_on(async {
                    let file = AsyncFileDialog::new()
                        .add_filter("GPX files", &["gpx"])
                        .add_filter("All files", &["*"])
                        .set_title("Open GPX file")
                        .pick_file()
                        .await;

                    let Some(file) = file else {
                        log::warn!("GPX file picker cancelled");
                        return Ok(None);
                    };
                    let path = file.path().to_string_lossy().to_string();
                    let data = journey::read_gpx_file_data(&path)?;
                    log::info!(
                        "Selected GPX file '{}' ({} bytes)",
                        file.file_name(),
                        data.len()
                    );
                    Ok(Some(data))
                });
                sender.send(result);
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let result = async {
                    let file = AsyncFileDialog::new()
                        .add_filter("GPX files", &["gpx"])
                        .add_filter("All files", &["*"])
                        .set_title("Open GPX file")
                        .pick_file()
                        .await;

                    let Some(file) = file else {
                        log::warn!("GPX file picker cancelled");
                        return Ok(None);
                    };
                    if file.inner().size() > journey::MAX_GPX_FILE_BYTES as f64 {
                        return Err(format!(
                            "GPX file is too large (maximum {} MB)",
                            journey::MAX_GPX_FILE_BYTES / (1024 * 1024)
                        ));
                    }
                    let data = read_web_file(file.inner()).await?;
                    log::info!(
                        "Selected GPX file '{}' ({} bytes)",
                        file.file_name(),
                        data.len()
                    );
                    Ok(Some(data))
                }
                .await;
                sender.send(result);
            });
        }

        self.file_promise = Some(promise);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_file(&mut self, ctx: &Context, file_path: String) {
        match journey::load_gpx_file(&file_path) {
            Ok(gpx) => {
                log::info!("Loaded GPX file '{}'", file_path);
                self.load(ctx, gpx, None);
            }
            Err(error) => {
                log::error!("Failed to load GPX file '{}': {}", file_path, error);
                self.error_message = Some(error);
            }
        }
    }

    fn load_journey_string(&mut self, ctx: &Context, journey_string: String) {
        log::info!(
            "Importing encoded journey string ({} chars)",
            journey_string.len()
        );

        match journey::import(&journey_string) {
            Ok((name, gpx)) => {
                log::info!("Decoded journey '{}' and preparing to load it", name);
                self.load(ctx, gpx, Some(name));
            }
            Err(error) => {
                log::error!("Failed to import journey string: {}", error);
                self.error_message = Some(error);
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() && self.file_promise.is_some() {
            self.error_message = Some("A GPX file is already loading".to_string());
            return;
        }
        if dropped_files.len() > 1 {
            self.error_message = Some("Please drop one GPX file at a time".to_string());
            return;
        }
        for file in dropped_files {
            let path = file.path();
            if path
                .extension()
                .is_none_or(|ext| ext.to_string_lossy().to_lowercase() != "gpx")
            {
                self.error_message = Some("Only GPX files are supported".to_string());
                continue;
            }

            let filename = path.to_string_lossy().to_string();
            log::info!("Dropped GPX file: {}", filename);

            #[cfg(not(target_arch = "wasm32"))]
            self.load_file(ctx, filename);

            #[cfg(target_arch = "wasm32")]
            {
                let Some(web_file) = file.web_file().cloned() else {
                    self.error_message = Some("Unable to access dropped GPX file".to_string());
                    continue;
                };
                if web_file.size() > journey::MAX_GPX_FILE_BYTES as f64 {
                    self.error_message = Some(format!(
                        "GPX file is too large (maximum {} MB)",
                        journey::MAX_GPX_FILE_BYTES / (1024 * 1024)
                    ));
                    continue;
                }

                let (sender, promise) = Promise::new();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = read_web_file(&web_file).await.map(Some);
                    sender.send(result);
                });
                self.file_promise = Some(promise);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn read_web_file(file: &web_sys::File) -> Result<Vec<u8>, String> {
    if file.size() > journey::MAX_GPX_FILE_BYTES as f64 {
        return Err(format!(
            "GPX file is too large (maximum {} MB)",
            journey::MAX_GPX_FILE_BYTES / (1024 * 1024)
        ));
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .map_err(|_| "Failed to read GPX file".to_string())?;
    let data = js_sys::Uint8Array::new(&buffer).to_vec();
    if data.len() > journey::MAX_GPX_FILE_BYTES {
        return Err(format!(
            "GPX file is too large (maximum {} MB)",
            journey::MAX_GPX_FILE_BYTES / (1024 * 1024)
        ));
    }
    Ok(data)
}

fn top_action_button(
    ui: &mut Ui,
    label: &str,
    width: f32,
    height: f32,
    text_size: f32,
    fill: Color32,
    border: Color32,
) -> bool {
    ui.add_sized(
        Vec2::new(width, height),
        Button::new(RichText::new(label).size(text_size).color(Color32::WHITE))
            .fill(fill)
            .stroke(Stroke::new(1.5, border)),
    )
    .clicked()
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

fn qr_to_texture(ctx: &egui::Context, data: &str) -> Option<TextureHandle> {
    //println!("length = {}", data.len());
    //println!("{}", &data);

    if let Ok(code) = QrCode::new(data) {
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

        Some(ctx.load_texture("qrcode", image, TextureOptions::NEAREST))
    } else {
        None
    }
}

fn openstreetmap_tiles(ctx: &egui::Context) -> HttpTiles {
    #[cfg(not(target_arch = "wasm32"))]
    let options = walkers::HttpOptions {
        user_agent: Some(walkers::HeaderValue::from_static("JourneyView/0.1")),
        cache: std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .map(|home| home.join(".cache/journeyview/map-tiles")),
        ..Default::default()
    };
    #[cfg(target_arch = "wasm32")]
    let options = walkers::HttpOptions::default();

    let tiles = HttpTiles::with_options(OpenStreetMap, options, ctx.clone());
    log::info!("Loaded map tiles");
    tiles
}

struct RouteLayer<'a> {
    segments: &'a [JourneySegment],
}

impl Plugin for RouteLayer<'_> {
    fn run(
        self: Box<Self>,
        ui: &mut Ui,
        _response: &egui::Response,
        projector: &Projector,
        _map_memory: &MapMemory,
    ) {
        for segment in self.segments {
            let points: Vec<egui::Pos2> = segment
                .points
                .iter()
                .filter(|point| point[0].is_finite() && point[1].is_finite())
                .map(|point| projector.project(lon_lat(point[0], point[1])).to_pos2())
                .collect();

            if points.len() >= 2 {
                ui.painter().add(egui::Shape::line(
                    points.clone(),
                    Stroke::new(6.0, Color32::from_black_alpha(190)),
                ));
                ui.painter().add(egui::Shape::line(
                    points.clone(),
                    Stroke::new(3.5, segment.color),
                ));
            }

            if let Some(start) = points.first() {
                ui.painter().circle_filled(*start, 6.0, Color32::WHITE);
                ui.painter()
                    .circle_filled(*start, 4.0, Color32::from_rgb(33, 150, 243));
            }
            if let Some(end) = points.last() {
                ui.painter().circle_filled(*end, 6.0, Color32::WHITE);
                ui.painter()
                    .circle_filled(*end, 4.0, Color32::from_rgb(244, 67, 54));
            }
        }
    }
}

fn reset_button(ui: &mut Ui, rect: egui::Rect) -> egui::Response {
    let btn_size = Vec2::new(56.0, 36.0);
    let padding = Vec2::new(12.0, 12.0);
    let btn_rect = egui::Rect::from_min_size(
        egui::pos2(
            rect.right() - padding.x - btn_size.x,
            rect.bottom() - padding.y - btn_size.y,
        ),
        btn_size,
    );
    ui.put(
        btn_rect,
        Button::new(RichText::new("\u{1F504}").size(20.0))
            .fill(Color32::from_rgb(33, 150, 243))
            .stroke(Stroke::new(1.5, Color32::BLACK)),
    )
    .on_hover_text("Reset view")
}

fn route_center(segments: &[JourneySegment]) -> Position {
    route_bounds(segments)
        .map(|(min_lon, min_lat, max_lon, max_lat)| {
            lon_lat((min_lon + max_lon) / 2.0, (min_lat + max_lat) / 2.0)
        })
        .unwrap_or_else(|| lon_lat(0.0, 0.0))
}

fn route_bounds(segments: &[JourneySegment]) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for point in segments.iter().flat_map(|segment| segment.points.iter()) {
        let [lon, lat] = *point;
        if !lon.is_finite() || !lat.is_finite() {
            continue;
        }
        bounds = Some(match bounds {
            Some((min_lon, min_lat, max_lon, max_lat)) => (
                min_lon.min(lon),
                min_lat.min(lat),
                max_lon.max(lon),
                max_lat.max(lat),
            ),
            None => (lon, lat, lon, lat),
        });
    }
    bounds
}

fn fit_map_to_segments(
    memory: &mut MapMemory,
    center: &mut Position,
    segments: &[JourneySegment],
    size: Vec2,
) {
    let Some((min_lon, min_lat, max_lon, max_lat)) = route_bounds(segments) else {
        *center = lon_lat(0.0, 0.0);
        memory.center_at(*center);
        let _ = memory.set_zoom(1.0);
        log::warn!("Route bounds were empty when fitting map");
        return;
    };

    *center = route_center(segments);
    memory.center_at(*center);

    let width = (size.x - 96.0).max(64.0) as f64;
    let height = (size.y - 96.0).max(64.0) as f64;
    let x_span = ((max_lon - min_lon).abs() / 360.0).max(f64::EPSILON);
    let y_span = (mercator_y(max_lat) - mercator_y(min_lat))
        .abs()
        .max(f64::EPSILON);

    let zoom_x = (width / (256.0 * x_span)).log2();
    let zoom_y = (height / (256.0 * y_span)).log2();
    let zoom = if (max_lon - min_lon).abs() < 1e-9 && (max_lat - min_lat).abs() < 1e-9 {
        16.0
    } else {
        zoom_x.min(zoom_y).clamp(1.0, 18.0)
    };
    let _ = memory.set_zoom(zoom);
}

fn mercator_y(latitude: f64) -> f64 {
    let latitude = latitude.clamp(-85.051_128_78, 85.051_128_78).to_radians();
    (1.0 - (latitude.tan() + 1.0 / latitude.cos()).ln() / std::f64::consts::PI) / 2.0
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
                    self.elevation_panel(ui);
                } else {
                    self.map_panel(ui);
                }
            }
            Mode::Import => self.import_panel(ui),
            Mode::Export => self.export_panel(ui),
            Mode::Info => self.info_panel(ui),
        });

        let ctx = ui.ctx().clone();

        // check on the async file loading dialog
        if let Some(promise) = &self.file_promise
            && let Some(result) = promise.ready()
        {
            match result {
                Ok(Some(data)) => match journey::load_gpx_data(data) {
                    Ok(gpx) => self.load(&ctx, gpx, None),
                    Err(error) => {
                        log::error!("Failed to load GPX file: {}", error);
                        self.error_message = Some(error);
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    log::error!("Failed to read GPX file: {}", error);
                    self.error_message = Some(error.clone());
                }
            }
            self.file_promise = None;
        }

        // check for any dropped files
        self.handle_dropped_files(&ctx);
    }
}
