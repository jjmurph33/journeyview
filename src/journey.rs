use base64::{Engine, engine::general_purpose::URL_SAFE};
use flexpolyline::Polyline;
use geo_types::Point;
use gpx::{Gpx, GpxVersion, Link, Track, TrackSegment, Waypoint};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;

#[derive(Serialize, Deserialize)]
pub struct Journey {
    name: String,
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
    let coordinates: Vec<(f64, f64, f64)> = gpx
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .flat_map(|segment| segment.points.iter())
        .map(|p| (p.point().y(), p.point().x(), p.elevation.unwrap_or(0.0)))
        .collect();
    let polyline = Polyline::Data3d {
        coordinates,
        precision2d: flexpolyline::Precision::Digits6,
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
    // name and polyline are joined with a "|" character and then compressed and base64 encoded
    //TODO: check for "|" character in the name when importing or renaming
    let data = format!("{}|{}", name, polyline);
    let bytes = data.as_bytes();
    // 22 = maximum compression
    match zstd::encode_all(bytes, 22) {
        Ok(compressed) => return URL_SAFE.encode(&compressed),
        Err(e) => eprint!("Error compressing data: {}", e),
    }
    String::new()
}

pub fn decode(encoded: &str) -> Option<Journey> {
    match URL_SAFE.decode(encoded) {
        Ok(bytes) => match zstd::decode_all(bytes.as_slice()) {
            Ok(decompressed) => match String::from_utf8(decompressed) {
                Ok(decoded) => {
                    //println!("decoded:\n{}\n", decoded);
                    if let Some((name, polyline)) = decoded.split_once("|") {
                        //println!("name = {}", name);
                        //println!("polyline = {}", polyline);
                        let journey = Journey {
                            name: name.to_string(),
                            polyline: polyline.to_string(),
                        };
                        return Some(journey);
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
    let journey_string = encode(name, &polyline);
    if journey_string.len() > 2000 {
        //TODO: try to reduce the polyline
        String::new()
    } else {
        journey_string
    }
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

// returns Vec of JourneySegments for the plot
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

// returns Vec of JourneySegments for the elevation plot
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

const SAMPLE_JOURNEY: &str = r#"KLUv_QCIVSoAqnUgFCUAScwcULHvkvC37OEHp0Cb4NwIOWhTUlKCi0r5LuL_nf7__y8hPAE5ATYBB9YGKJguZGfEC4uenNkuMPWOqSa5pkykF8Z2L9scOCZh21NHFLGP6eF74wk5soR1aw-5QHS_E6Q3ZaQfwsA-QRvJrUTsN1nI3xZIr0lsncHrn8ZgH7YDFBcGvoVXK8PadmW1dE2tX42t6F20QT44lolxZfXSlP9pEf0aJeBWmIL9V7aAm_QsjezbDv6xnNKt5P4htookyg54F3z2rQO50BSLxxZEr4DPgUTRZcRe1reeFl3gyDLFTeOp1DQL3jSaoJFoMZ6kp5qwYIqpbApfMgmtYVPMLLTSmnZhpwNP8S1qprFvhifJrd3wt2QkRUQp0k-7tSOy2Ucgo6011n9JxPshD2llVxbbJDS1TtFCKiZF5Jhi_5NOkrSphh9BK_KDWpHCyksVHUFALYZZGkk6EMfZkIANFlAuBDBZBAx8jqR9acf4MGJfg4ToE9hEbNjCs8aSiAnbRY-y-c3uo_Chb8Iqt-Gtr11moWSKq0k7xs72tI5lhh-JAHfN4DNp4d2CLwst45DQJ4KhZ2wnDW08e3x8djDgSSTt-GmEGPqDztoPPFlow5Zsik4aW3ncvso-thHa9SPQED1Lm0iKYGQXk04tFQ5I76KG5QI7umHh5B9CQfQMVtIe2EG9MLePHS9tA_5tkiRNWUknoJNPQ0M_S6_8zfspOZEYlDTsMyjJv-C2kAjfkvXUfgUjjQW1ld6Nt8YfmJIakx4OnlmrFRQlRF7Ayj-KUvsYZSJHHPw7ZLMPw-YZwdsvw4VsjDVpm8canfoEfUluapHgUyeCD78RjqQamNrkulkCbjWwwpdUq7HPQ1ubWlP_lBYirPgksbaExI4rywEJZsMaonx09ZTjijXBEBtwK34ouHIIL1S14y20DlyIM08Id5wZS3wvfjBw0lmUh76UrYWiBC3JtHCCgR06zTHyYQeXHTqgIclgQ4C9GMKG6MML4Yqdayj6SXau6BBC73DC7igEn9NI4p7QSh0qwHiOHQypUwaeEDsKtiynL9EHL7h0lB9NPIWDk-hZGO_gSgydS4L-BCP8FW4sPWrhPwn4HwPcnF78TgD1LClRSriAC-FNOWOKFQ9W3I5aLlW5uSfcQk9gwd-J2A0owWcQgG5JBxtaCz6UGPaljTcezDB3i3gCflJ1sMWLK7WngAuuGjOcFW78-cAGOuI4cfMQ8EXAOq45P5aA7NiIvwUjvM8yDx0m6PdGYCWNg0vdM5wZEzAhRvDqTnjLtLwVfUQZ9EUiIfRMqei50XPyoZboH2l4sD84SUVsop-CVvQQPOhPBKBfCWFb9NFH5A6MoD-aJT68crZwFnkiL-1HK_8EXuiQfJSd7aKuJEkA3YwzdycfLnGZYWebKVfNPenZzjv3aiW8kzUgV-uo5bFFK4YwJ9Z5__CE92KJd9YZ-9WO97MTZgNzy9dyDE50FfGjED-HlOdQTvomDNQdy_iOgP-zQL9rb3SL8T_w4G-Q1CHk9DPM2BOaeJ5AA3KiIRFL1vDcQYfbsVykib44HX5wuYWiTSznnqlF-9kM2bGTaK-GvJ8seIoEWvg7xPS5A8gvIfBegpdKz0r4q-zQoZS8r7B6kgM7VYizdEsN75N27A9XIsUjKexcTngtbGG38LUGdygdb8lmCn_sE-njH_eHGlSdrpQtDMGv8KFQCQcYCIB9QSnEmhco4SrOzayF3hpmLBkDEaPHOHHEVepugGDi0ya2QoxZwK0CTJSv0glnNVdhOBCCw18LguTOKA=="#;
