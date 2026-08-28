use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use flexpolyline::Polyline;
use geo_types::Point;
use gpx::{Gpx, GpxVersion, Link, Track, TrackSegment, Waypoint};
use std::fs;
use std::io::BufReader;

//#[derive(Serialize, Deserialize)]
pub struct Journey {
    name: String,
    //version: String,
    polyline: String,
}

pub struct JourneySegment {
    pub points: Vec<[f64; 2]>,
    pub name: String,
    pub color: egui::Color32,
}

pub fn load_gpx_file(file_path: &str) -> Result<Gpx, String> {
    match fs::File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match gpx::read(reader) {
                Ok(gpx) => {
                    //println!("{:?}", gpx);
                    Ok(gpx)
                }
                Err(e) => Err(format!("Failed to parse GPX file: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to open file: {}", e)),
    }
}

fn to_polyline(gpx: &Gpx) -> String {
    // (lat,lon,elev)
    let mut coordinates: Vec<(f64, f64, f64)> = gpx
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .flat_map(|segment| segment.points.iter())
        .map(|p| (p.point().y(), p.point().x(), p.elevation.unwrap_or(0.0)))
        .collect();

    while coordinates.len() > 500 {
        // remove every other point to reduce the size of the polyline
        coordinates = coordinates
            .iter()
            .enumerate()
            .filter_map(|(i, val)| if i % 2 == 0 { Some(*val) } else { None })
            .collect();
    }

    let polyline = Polyline::Data3d {
        coordinates,
        precision2d: flexpolyline::Precision::Digits5,
        precision3d: flexpolyline::Precision::Digits0,
        type3d: flexpolyline::Type3d::Elevation,
    };

    polyline.encode().unwrap_or_default()
}

fn from_polyline(polyline: &str) -> Gpx {
    let mut gpx = Gpx {
        version: GpxVersion::Gpx11,
        ..Default::default()
    };
    let decoded = Polyline::decode(polyline).unwrap();
    if let Polyline::Data3d { coordinates, .. } = decoded {
        let mut segment = TrackSegment::new();
        segment.points = coordinates
            .iter()
            .map(|c| {
                let mut waypoint = Waypoint::new(Point::new(c.1, c.0));
                waypoint.elevation = Some(c.2);
                waypoint
            })
            .collect();
        let mut track = Track::new();
        track.segments.push(segment);
        gpx.tracks.push(track);
    };
    gpx
}

fn encode(name: &str, polyline: &str) -> String {
    // name and polyline along with the version number are joined with a "|" character and then compressed and base64 encoded
    let version = 1; // version number: change this if the format changes
    let name = name.replace("|", "-"); // replace any "|" characters in the name
    let data = format!("{}|{}|{}", name, version, polyline);
    match zstd::encode_all(data.as_bytes(), 0) {
        Ok(compressed) => URL_SAFE_NO_PAD.encode(&compressed),
        Err(e) => {
            eprint!("Error compressing data: {}", e);
            String::new()
        }
    }
}

pub fn decode(encoded: &str) -> Option<Journey> {
    match URL_SAFE_NO_PAD.decode(encoded) {
        Ok(bytes) => match zstd::decode_all(bytes.as_slice()) {
            Ok(decompressed) => match String::from_utf8(decompressed) {
                Ok(decoded) => {
                    let parts: Vec<&str> = decoded.split("|").collect();
                    if parts.len() < 3 {
                        return None;
                    } else {
                        let version = parts[1].to_string();
                        if version == "1" {
                            let journey = Journey {
                                name: parts[0].to_string(),
                                //version,
                                polyline: parts[2].to_string(),
                            };
                            return Some(journey);
                        } else {
                            eprintln!("Version {} not supported", version);
                            return None;
                        }
                    }
                }
                Err(e) => eprintln!("Error decoding imported data: {}", e),
            },
            Err(e) => eprintln!("Error decompressing imported data: {}", e),
        },
        Err(e) => eprintln!("Error reading imported data: {}", e),
    }
    None
}

pub fn export(name: &str, gpx: &Gpx) -> String {
    let polyline = to_polyline(gpx);
    encode(name, &polyline)
}

pub fn import(journey_string: &str) -> Result<(String, Gpx), String> {
    if let Some(journey) = decode(journey_string) {
        let name = journey.name.clone();
        let metadata = gpx::Metadata {
            name: Some(name.clone()),
            ..Default::default()
        };
        let mut gpx = from_polyline(&journey.polyline);
        gpx.metadata = Some(metadata);
        Ok((name, gpx))
    } else {
        Err(String::from("Not a valid Journey"))
    }
}

pub fn import_sample() -> Result<(String, Gpx), String> {
    import(SAMPLE_JOURNEY)
}

pub fn name_from_gpx(gpx: &Gpx) -> String {
    if let Some(metadata) = &gpx.metadata
        && let Some(name) = &metadata.name
    {
        return name.clone();
    }
    String::new()
}

pub fn info(gpx: &Gpx) -> String {
    let mut info = String::new();

    info.push_str(&format!("Version: {:?}\n", gpx.version));

    if let Some(creator) = &gpx.creator {
        info.push_str(&format!("Creator: {}\n", creator));
    }

    if let Some(metadata) = &gpx.metadata {
        if let Some(name) = &metadata.name {
            info.push_str(&format!("Name: {}\n", name));
        }
        if let Some(description) = &metadata.description {
            info.push_str(&format!("Description: {}\n", description));
        }
        if let Some(author) = &metadata.author {
            if let Some(name) = &author.name {
                info.push_str(&format!("Author: {}\n", name));
            }
            if let Some(email) = &author.email {
                info.push_str(&format!("Email: {}\n", email));
            }
        }
        for link in &metadata.links {
            info.push_str(&link_info(link));
        }
        if let Some(time) = &metadata.time {
            info.push_str(&format!("Time: {}\n", time.format().unwrap_or_default()));
        }
        if let Some(keywords) = &metadata.keywords {
            info.push_str(&format!("Keywords: {}\n", keywords));
        }
        if let Some(copyright) = &metadata.copyright {
            info.push_str(&format!(
                "Copyright: {}\n",
                copyright.author.as_ref().unwrap_or(&String::new())
            ));
            if let Some(year) = &copyright.year {
                info.push_str(&format!("Copyright Year: {}\n", year));
            }
            if let Some(license) = &copyright.license {
                info.push_str(&format!("Copyright License: {}\n", license));
            }
        }
        if let Some(bounds) = &metadata.bounds {
            info.push_str(&format!(
                "Bounds: minlat={}, minlon={}, maxlat={}, maxlon={}\n",
                bounds.min().y,
                bounds.min().x,
                bounds.max().y,
                bounds.max().x
            ));
        }
    }

    info.push_str(&format!("Waypoints: {}\n", gpx.waypoints.len()));

    for track in &gpx.tracks {
        if let Some(name) = &track.name {
            info.push_str(&format!("Track Name: {}\n", name));
        }
        if let Some(comment) = &track.comment {
            info.push_str(&format!("Track Comment: {}\n", comment));
        }
        if let Some(description) = &track.description {
            info.push_str(&format!("Track Description: {}\n", description));
        }
        if let Some(source) = &track.source {
            info.push_str(&format!("Track Source: {}\n", source));
        }
        for link in &track.links {
            info.push_str(&link_info(link));
        }
        if let Some(type_) = &track.type_ {
            info.push_str(&format!("Track Type: {}\n", type_));
        }
        if let Some(number) = &track.number {
            info.push_str(&format!("Track Number: {}\n", number));
        }
        info.push_str(&format!("Track Segments: {}\n", track.segments.len()));
        for (i, segment) in track.segments.iter().enumerate() {
            info.push_str(&format!(
                "\tSegment {}: {} waypoints\n",
                i + 1,
                segment.points.len()
            ));
        }
    }

    for route in &gpx.routes {
        if let Some(name) = &route.name {
            info.push_str(&format!("Route Name: {}\n", name));
        }
        if let Some(comment) = &route.comment {
            info.push_str(&format!("Route Comment: {}\n", comment));
        }
        if let Some(description) = &route.description {
            info.push_str(&format!("Route Description: {}\n", description));
        }
        if let Some(source) = &route.source {
            info.push_str(&format!("Route Source: {}\n", source));
        }
        for link in &route.links {
            info.push_str(&link_info(link));
        }
        if let Some(number) = &route.number {
            info.push_str(&format!("Route Number: {}\n", number));
        }
        if let Some(type_) = &route.type_ {
            info.push_str(&format!("Route Type: {}\n", type_));
        }
        for waypoint in &route.points {
            info.push_str(&waypoint_info(waypoint));
        }
    }
    info
}

fn waypoint_info(waypoint: &Waypoint) -> String {
    let mut info = String::new();
    info.push_str(&format!(
        "({}),({})\n",
        waypoint.point().y(),
        waypoint.point().x()
    ));
    if let Some(elevation) = &waypoint.elevation {
        info.push_str(&format!("Elevation (m): {}\n", elevation));
    }
    if let Some(speed) = &waypoint.speed {
        info.push_str(&format!("Speed (m/s): {}\n", speed));
    }
    if let Some(time) = &waypoint.time {
        info.push_str(&format!("Time: {}\n", time.format().unwrap_or_default()));
    }
    if let Some(name) = &waypoint.name {
        info.push_str(&format!("Name: {}\n", name));
    }
    if let Some(comment) = &waypoint.comment {
        info.push_str(&format!("Comment: {}\n", comment));
    }
    if let Some(description) = &waypoint.description {
        info.push_str(&format!("Description: {}\n", description));
    }
    if let Some(source) = &waypoint.source {
        info.push_str(&format!("Source: {}\n", source));
    }
    for link in &waypoint.links {
        info.push_str(&link_info(link));
    }
    if let Some(symbol) = &waypoint.symbol {
        info.push_str(&format!("Symbol: {}\n", symbol));
    }
    if let Some(type_) = &waypoint.type_ {
        info.push_str(&format!("Type: {}\n", type_));
    }
    if let Some(geoidheight) = &waypoint.geoidheight {
        info.push_str(&format!("Geoid Height: {}\n", geoidheight));
    }
    if let Some(fix) = &waypoint.fix {
        info.push_str(&format!("Fix: {:?}\n", fix));
    }
    if let Some(sat) = &waypoint.sat {
        info.push_str(&format!("Sat: {}\n", sat));
    }
    if let Some(hdop) = &waypoint.hdop {
        info.push_str(&format!("HDOP: {}\n", hdop));
    }
    if let Some(vdop) = &waypoint.vdop {
        info.push_str(&format!("VDOP: {}\n", vdop));
    }
    if let Some(pdop) = &waypoint.pdop {
        info.push_str(&format!("PDOP: {}\n", pdop));
    }
    if let Some(dgps_age) = &waypoint.dgps_age {
        info.push_str(&format!("DGPS Age: {}\n", dgps_age));
    }
    if let Some(dgpsid) = &waypoint.dgpsid {
        info.push_str(&format!("DGPS ID: {}\n", dgpsid));
    }
    info
}

fn link_info(link: &Link) -> String {
    let mut info = String::new();
    info.push_str(&format!("Link: {}\n", link.href));
    if let Some(text) = &link.text {
        info.push_str(&format!("Link Text: {}\n", text));
    }
    if let Some(type_) = &link.type_ {
        info.push_str(&format!("Link Type: {}\n", type_));
    }
    info
}

// minimum elevation of all tracks (in meters)
pub fn min_elevation(gpx: &Gpx) -> f64 {
    let mut min = f64::MAX;
    for track in &gpx.tracks {
        for segment in &track.segments {
            for waypoint in &segment.points {
                if let Some(elevation) = waypoint.elevation
                    && elevation < min
                {
                    min = elevation;
                }
            }
        }
    }
    min
}

// maximum elevation of all tracks (in meters)
pub fn max_elevation(gpx: &Gpx) -> f64 {
    let mut max = f64::MIN;
    for track in &gpx.tracks {
        for segment in &track.segments {
            for waypoint in &segment.points {
                if let Some(elevation) = waypoint.elevation
                    && elevation > max
                {
                    max = elevation;
                }
            }
        }
    }
    max
}

// total distance of all tracks (in kilometers)
pub fn distance(gpx: &Gpx) -> f64 {
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

pub fn km_to_mi(value: f64) -> f64 {
    value * 0.621371
}

pub fn m_to_ft(value: f64) -> f64 {
    value * 3.28084
}

// returns JourneySegments for the map plot
pub fn plot_segments(gpx: &Gpx) -> Vec<JourneySegment> {
    let mut segments = Vec::new();
    for (i, track) in gpx.tracks.iter().enumerate() {
        for seg in &track.segments {
            let points = seg
                .points
                .iter()
                .map(|p| [p.point().x(), p.point().y()])
                .collect();
            let name = track
                .name
                .clone()
                .unwrap_or_else(|| format!("Track {}", i + 1));
            let color = egui::Color32::from_rgb(66, 244, 133);
            let segment = JourneySegment {
                points,
                name,
                color,
            };
            segments.push(segment);
        }
    }
    segments
}

// returns JourneySegments for the elevation plot
pub fn elevation_segments(gpx: &Gpx) -> Vec<JourneySegment> {
    let mut segments = Vec::new();
    let mut distance = 0.0;
    let mut prev: Option<(f64, f64)> = None; // (lat, lon)
    for (i, track) in gpx.tracks.iter().enumerate() {
        for seg in &track.segments {
            let mut points: Vec<[f64; 2]> = Vec::new();
            for p in &seg.points {
                let lat = p.point().y();
                let lon = p.point().x();
                if let Some((prev_lat, prev_lon)) = prev {
                    distance += haversine_distance(prev_lat, prev_lon, lat, lon);
                }
                prev = Some((lat, lon));
                let x = km_to_mi(distance);
                let y = m_to_ft(p.elevation.unwrap_or(0.0));
                points.push([x, y]);
            }
            let points = points.into_iter().collect();
            let name = track
                .name
                .clone()
                .unwrap_or_else(|| format!("Track {}", i + 1));
            let color = egui::Color32::from_rgb(0, 0, 255);
            let segment = JourneySegment {
                points,
                name,
                color,
            };
            segments.push(segment);
        }
    }
    segments
}

const SAMPLE_JOURNEY: &str = r#"KLUv_QCIHRsAqkXMCyMwa4EeWMdxXddldVVV6ANJgD_WLdq2bWspqQ08-GQwNBRUcLwAsgCvAIkfUSVuYYwJL_h0rjhVRI7Rk3j4wCJGHWecM-woh_GCcqUV05OVRi56AU_CFIzbOJ7JUdHZoZSIeIExnIqB3iDGMG-OYtA5kuKFd6Yr53ABdoovL3zkhvUJPSncET1OqKhWCTzYwsiSw814inlUwUXhWNGHU5hkj0IdkR2QGcXYwTEFokQ5g7ooEMM8UadwrgpP3GEH3sMpRTFucKycTdiyw_cWS8Nj8PCAQTkIECAAC-AIAiI0DwQYAAMXK6OKcufbZg-z3nQzfSLhkDokziTUeiEZS9QLO4ibhPOCpBhV8UFnqK7xJLBG0Ut1KEqkc_IUC8YEb54fDTG_jVtH7HBY03xKxA-vGt6mGZTspK9UvLBH-cM8atZRFjGJcYLkzR8FOW5YxA2KKw75AERwQiYyXuAiAxEXYJsg1TDnBbfKUY1SM6o4TzFZVNAe6hxnp3uC2FQwnjY9Fc4J9HhYoVjhqFh0sBjPz-OoiZTiZosgmJD_RHrEa7qKdiTW6Un0vWxN35K8tA89K4lH-RGPGLukxAOekf7auO4yw0p1deVXGVovnJJskuglPRtYLdviAb0KvBgCYoQ1MzBezi_-xvqF21BL9BssKahUD0ZQWvZrswIz8nhRTFliBcXLuDjpN58JoOSKVzPt_FMkn6unfEkiIFOYqj8R188-B338kJih-TR5zX-xXzxJWQVyXcwIFhtsl-WEbdr-C298FxSr8F1_a_o-jkCM78JcexHnuk4bSco1Vlq6JbH0PTNsN5nqJ4XEtLU0Vr-akPfv9ZJLtpdvLTJR-gZNxnsqnXhP0qXtq6-KOqHfwqmeXjZxA_gVpeU08l27NqZ0If-IJV4yWhuP6dtG1-KLCfwv3vWwwn7ltXQzw_uaGEmr9ncjztHU0R6ZywPUI-mffIRINTXj7VoS0rrRlpIrt0VfEpvkWvKbfNr4EY1AnjACNSgQEEIUwx7gHSi872yBm-rnWQyLkr8wFHoyZyWs0Ebdsq7ADGVUY5cE_jMlqNy9Tol-wpUlt-1BZ_eeUWNQYEAtELUMkvESIHtoviTMGSi3jmkCtzprDREyVm4CzLKbW2gfm1bFMHqAZCUK"#;
