mod app;

use base64::{Engine, engine::general_purpose::URL_SAFE};
use eframe;
use flexpolyline::Polyline;
use geo_types::Point;
use gpx::GpxVersion;
use gpx::{Gpx, Metadata, Track, TrackSegment, Waypoint};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Journey {
    name: String,
    polyline: String,
}

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

    let journey = decode(SAMPLE_JOURNEY);
    let name = Some(journey.name);
    let mut metadata = Metadata::default();
    metadata.name = name;
    let mut gpx = from_polyline(&journey.polyline);
    gpx.metadata = Some(metadata);

    let polyline = to_polyline(&gpx).unwrap();
    let metadata = gpx.metadata.as_ref().unwrap();
    let name = metadata.name.as_ref().unwrap().clone();
    let journey_encoded = encode(&name, &polyline);
    println!("{}", journey_encoded);

    eframe::run_native(
        "GPX Viewer",
        options,
        Box::new(move |cc| {
            let app = app::App::new(cc, gpx);
            Ok(Box::new(app))
        }),
    )
}

fn to_polyline(data: &Gpx) -> Result<String, flexpolyline::Error> {
    // (lat,lon,elev)
    let coordinates: Vec<(f64, f64, f64)> = data
        .tracks
        .iter()
        .flat_map(|t| t.segments.iter())
        .flat_map(|seg| seg.points.iter())
        .map(|p| (p.point().y(), p.point().x(), p.elevation.unwrap_or(0.0)))
        .collect();
    let polyline = Polyline::Data3d {
        coordinates,
        precision2d: flexpolyline::Precision::Digits6,
        precision3d: flexpolyline::Precision::Digits0,
        type3d: flexpolyline::Type3d::Elevation,
    };
    polyline.encode()
}

fn from_polyline(data: &str) -> Gpx {
    let mut gpx: Gpx = Default::default();
    gpx.version = GpxVersion::Gpx11;
    let polyline = Polyline::decode(data).unwrap();
    if let Polyline::Data3d { coordinates, .. } = polyline {
        let mut segment = TrackSegment::new();
        segment.points = coordinates
            .iter()
            .map(|c| {
                let mut wpt = Waypoint::new(Point::new(c.1, c.0));
                wpt.elevation = Some(c.2);
                wpt
            })
            .collect();
        let mut track = Track::new();
        track.segments.push(segment);
        gpx.tracks.push(track);
    };
    gpx
}

fn encode(name: &str, polyline: &str) -> String {
    let journey = Journey {
        name: name.to_string(),
        polyline: polyline.to_string(),
    };
    let bytes = postcard::to_allocvec(&journey).unwrap();
    URL_SAFE.encode(&bytes)
}

fn decode(encoded: &str) -> Journey {
    let bytes = URL_SAFE.decode(encoded).unwrap();
    postcard::from_bytes(&bytes).unwrap()
}

const SAMPLE_JOURNEY: &str = r#"B010IEJsdWWiD0IyQnMxMXAxQ3IweG1tRWlicUdMRGxDa0xBa0Q2SEFrQnFJQm9Ec0tLZ0RnSUltRG1IQUxnTEM0RHVIRWlHMkNFbUV3R0NrQ3VIRTZDNkhJOEN5SUdtR29HRC1COElFMEM4SEdzRWdJS3VEcUxLLUNzSUJzRWtIS2dGLUdJbUctR0FzSHVJYWdFNEdEb0d3RE0tRmlHQm9HMkNDeUZ1R0NpRmlGTTJHaUlLZ0ZtRUFxRDJHTzBINENEb0NpSU8wRmlGRXFGNEdNZ0dzSEV5RThGRUR1S1EzQjRKQWVtSU1Mb01DYWtJRTNCeUpTRXVLQWpFOEpJdkNrSENnQ21JSVFzSkVxRGtKS3NCc0pFNEV3SUltRThGRXVGd0VCMkQwSkc4RWdJSXVHeURFd0ZtQ0NtSDZGQjBEeUpBa0ZvRUlrRG9IQ29Fd0dFb0YtREdsQmtJQk55TFNFbUpFZm9KSWxCLUhFbUMwSktlc0lDNEMwSUU4Q2tISzBEMklFWThKR3FDZ0pLMkhJR0c4TUM2RWtIRTJFcUZCZ0ZzR0k2RG1JRThEc0dJcUdnR0VtS21IUTBFeUVFbUZ5REEtRG9IS29EbUhJaUY0R0UtSGdHTTRIcUNHaUcyQ0M0Ry1MU29GNkZFd0Z6QkMyRW9GS2dId0JBNERpSUlpR29DRWtHNkNJbUdrREN5RndCT2tIVEUwR3ZDRWlGNURDcUZxRUk2RjhDR2dIMkJFaUdGQ3lHY0lpSEJHbUkwRUtvRm1HQzRGLUVJc0Y4RkkwQm1KRTBDNkpJUThIQS1ELUpFaUR1Skl3QjRIQTJEdUxHa0UtSVE2RGdPS2lGNklPc0ZzRUd3RWlNT3FCd0lFaUlrS0dJMktNc0J5SUNyQi1JRTVDdU5LdEN1SUdIc0lDbERrSkc1Qm9KQzFCMkhDd0NnS0tLc0pJU3NLSV9GSkV2RnlEQjFGNkVLakItSUFkOEpJdEJrT0dfRjRGRTVEeUpLYTRLR2dEZ01FcUJxS0EwRGtKQTVCOElHekQ4R0VoRXdKRXZKd0VPdER1R0N4RjBETXRGNkhHbkdRSTdGN0JHM0Z6REtySlNRbkloQkluSDBET3RFbUZJN0Y4Q0k3QzZORTdFeUZNekhNS25JZkd6RlRLM0YtRkVfRWdHRTFGbUhHdEdRSWpGaUZDN0UtRUVsR3NFRWxGMEVDakJnSkRoQmlKRUUwSUNFdUpBekUyRkFyRm9GSTRFNUZBMEI5SUEyQjBKRThGM0JGekRsR0RLMUpCTnZJQWlEbkhCN0M5TUI3QzlIQi1EcUlBa0l4Q0NzRnRFRDRFNUVGOEY3Q0IyRm5DRi1JektINk9yRVRzR0lEMkYxQkotRTNFRnFCeElIcUVfRkIyRWhGSG9IMUVSeUZiSjBGZ0JIcUdZRjZGSUgwRndDRi1Gb0RINEZkSm9DbE1Ga0ZfRERnSVJKOEV6RkYtRmhPSDBEbEdCbUM3SUJ4RTdHQjFDeEhCUjVKQnJCNUhGQTFJSDRCOUhCa0g5SEhzQ2pLSGtEMUdGakIzTEZpRm5GRHNGcEVCNEZyQ0F1Q25IRFAxSUZsRHZHRlkzSUZjM0lCa0QzS0ZzQmpJRnFCdElEMkN2SEJtQ2xRSk54SkF0QjFMSjlEM0ZEMUN2TUZoRHJJRHZFeEZMMUY3Q0hmcklEdkR6S05fRnZJTlg3SERqRDlISHJEektBRHZJQnBEMUhGdEJ6SEI3QnpMSHpDdElBN0d2SEZqRnBFRjNEcEdCN0ZsRVA1Rk5DcEdTRF9GRUIxRjNDTnZHMUJMMUZ6Qk45RnhDQ3JHY0E3RWtGRGxHNkNBbkd0Qk5oRmxFSHJGbkNGekZwQ0huRjdFQl9FN0VKX0dqQ0Z6RjdFRnBIakZMaEZqRUI1RGxHRjdGOURGakZ2REpqR2hDQTVFNUZGOUZyRExoQmpJQnBGeEZEakcxREQ1RGhISDNJaERBM0VoSExuRXJHSDFDN0hINUVsRkJfRXhIRnhGeEVGdEN2SEZ1Q2hMQXdGM0hGekRnSEk1RGtIQ25GN0REekJfS0pqQmpJSnJDN0hCekNqSUZyRTNLRGxCcklKVHJJRk54SkJhdklEbUJ6SUZ4QjlMSmV0SUpwR2xFQXRFX0ZCbkUzR0R4RG5ISHZFOUVBN0V2RUIzRjFDRmxHOURBdkVfR0YxQjFISjVFckZBcEc5RkI1QjlISjlEN0hCekM5SEZuQm5JRHZCN0hESzVJSDZCaElGbUV6T0hHeklEZTNJSFR6SU5FX0hEZnRKRGdDN0hKdUJ2SUY3RDNGQWpHOUZOdkVsRkV2RjVGRnRFM0dIMUdsSEZuR25HSHJGbEVGdEYzSUoxRTlGRnBGakRGbEV4Rkp4RmpERDFFakZCNUV0RUZyRWpISHpIekVEMUUzS0p2R2pGRG5FbkdCbkN2S0hoQzFISG5GbkVEUHJLRm5FakdBekUxRkQzQ3BIQmxCOUlIakQzR0RsQzlIQW5GeEZIakhoQkNuRDNIQnFCX0hBaEJfSEQzRHhHQjdDbEhCUnRJRDZCcElBa0J1SUc5RThEQ3RCM0hGaEJ2SUY1Q2hJQXhCcElEaEMxSkE="#;
