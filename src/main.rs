mod app;

use eframe;
use gpx::Gpx;
use std::io::{BufReader, Cursor};

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

    //let gpx = Gpx::default();
    let cursor = Cursor::new(SAMPLE_GPX);
    let reader = BufReader::new(cursor);
    let gpx: Gpx = gpx::read(reader).unwrap();

    eframe::run_native(
        "GPX Viewer",
        options,
        Box::new(move |cc| {
            let app = app::App::new(cc, gpx);
            Ok(Box::new(app))
        }),
    )
}

const SAMPLE_GPX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="GPX Viewer Sample">
  <metadata>
    <name>Sample Mountain Hike</name>
  </metadata>
  <trk>
    <name>Mountain Trail</name>
    <trkseg>
      <trkpt lat="46.5000" lon="7.5000"><ele>1200</ele></trkpt>
      <trkpt lat="46.5010" lon="7.5020"><ele>1250</ele></trkpt>
      <trkpt lat="46.5025" lon="7.5035"><ele>1320</ele></trkpt>
      <trkpt lat="46.5040" lon="7.5050"><ele>1400</ele></trkpt>
      <trkpt lat="46.5055" lon="7.5060"><ele>1480</ele></trkpt>
      <trkpt lat="46.5070" lon="7.5065"><ele>1550</ele></trkpt>
      <trkpt lat="46.5085" lon="7.5060"><ele>1620</ele></trkpt>
      <trkpt lat="46.5100" lon="7.5050"><ele>1700</ele></trkpt>
      <trkpt lat="46.5115" lon="7.5035"><ele>1780</ele></trkpt>
      <trkpt lat="46.5125" lon="7.5020"><ele>1850</ele></trkpt>
      <trkpt lat="46.5130" lon="7.5000"><ele>1920</ele></trkpt>
      <trkpt lat="46.5128" lon="7.4980"><ele>1950</ele></trkpt>
      <trkpt lat="46.5120" lon="7.4965"><ele>1900</ele></trkpt>
      <trkpt lat="46.5110" lon="7.4955"><ele>1830</ele></trkpt>
      <trkpt lat="46.5095" lon="7.4950"><ele>1750</ele></trkpt>
      <trkpt lat="46.5080" lon="7.4955"><ele>1670</ele></trkpt>
      <trkpt lat="46.5065" lon="7.4965"><ele>1590</ele></trkpt>
      <trkpt lat="46.5050" lon="7.4975"><ele>1500</ele></trkpt>
      <trkpt lat="46.5035" lon="7.4985"><ele>1400</ele></trkpt>
      <trkpt lat="46.5015" lon="7.4995"><ele>1280</ele></trkpt>
      <trkpt lat="46.5000" lon="7.5000"><ele>1200</ele></trkpt>
    </trkseg>
  </trk>
</gpx>
"#;
