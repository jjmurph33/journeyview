use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use flexpolyline::Polyline;
use geo_types::Point;
use gpx::{Gpx, GpxVersion, Link, Track, TrackSegment, Waypoint};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::io::{BufReader, Cursor, Read};

pub const MAX_GPX_FILE_BYTES: usize = 10 * 1024 * 1024;
const MAX_JOURNEY_ENCODED_BYTES: usize = 64 * 1024;
const MAX_JOURNEY_COMPRESSED_BYTES: usize = 48 * 1024;
const MAX_JOURNEY_DECOMPRESSED_BYTES: usize = 256 * 1024;
const MAX_GPX_TRACKS: usize = 256;
const MAX_GPX_ROUTES: usize = 256;
const MAX_GPX_SEGMENTS: usize = 2_000;
const MAX_GPX_POINTS: usize = 100_000;
const MAX_GPX_LINKS: usize = 10_000;
const MAX_GPX_METADATA_BYTES: usize = 256 * 1024;
const MAX_GPX_XML_DEPTH: usize = 64;
const MAX_GPX_XML_ELEMENTS: usize = 400_000;
const MAX_GPX_UNKNOWN_ELEMENTS: usize = 10_000;

//#[derive(Serialize, Deserialize)]
#[derive(Debug)]
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

#[cfg(not(target_arch = "wasm32"))]
pub fn read_gpx_file_data(file_path: &str) -> Result<Vec<u8>, String> {
    let file = fs::File::open(file_path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mut data = Vec::new();
    file.take(MAX_GPX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| format!("Failed to read GPX file: {e}"))?;
    if data.len() > MAX_GPX_FILE_BYTES {
        return Err(format!(
            "GPX file is too large (maximum {} MB)",
            MAX_GPX_FILE_BYTES / (1024 * 1024)
        ));
    }
    Ok(data)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_gpx_file(file_path: &str) -> Result<Gpx, String> {
    let data = read_gpx_file_data(file_path)?;
    load_gpx_data(&data)
}

pub fn load_gpx_data(data: &[u8]) -> Result<Gpx, String> {
    if data.len() > MAX_GPX_FILE_BYTES {
        return Err(format!(
            "GPX file is too large (maximum {} MB)",
            MAX_GPX_FILE_BYTES / (1024 * 1024)
        ));
    }

    preflight_gpx(Cursor::new(data))?;
    let gpx =
        gpx::read(BufReader::new(data)).map_err(|e| format!("Failed to parse GPX data: {e}"))?;
    validate_gpx(&gpx)?;
    Ok(gpx)
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

fn validate_polyline_encoding(polyline: &str) -> Result<(), String> {
    let mut bytes = polyline.bytes().peekable();
    let version = decode_polyline_unsigned(&mut bytes)?;
    if version != 1 {
        return Err("Unsupported journey polyline version".to_string());
    }
    let header = decode_polyline_unsigned(&mut bytes)?;
    if header >= (1_u64 << 11) {
        return Err("Invalid journey polyline header".to_string());
    }

    let dimensions = if ((header >> 4) & 7) == 0 { 2 } else { 3 };
    let mut coordinates = [0_i64; 3];
    let mut point_count = 0usize;
    while bytes.peek().is_some() {
        for coordinate in coordinates.iter_mut().take(dimensions) {
            let delta = decode_polyline_signed(&mut bytes)?;
            *coordinate = coordinate
                .checked_add(delta)
                .ok_or_else(|| "Journey polyline coordinate overflow".to_string())?;
        }
        point_count += 1;
        if point_count > MAX_GPX_POINTS {
            return Err(format!(
                "Journey contains too many points (maximum {MAX_GPX_POINTS})"
            ));
        }
    }
    Ok(())
}

fn decode_polyline_signed<I: Iterator<Item = u8>>(bytes: &mut I) -> Result<i64, String> {
    let mut value = decode_polyline_unsigned(bytes)?;
    let negative = value & 1 != 0;
    value >>= 1;
    if negative {
        value = !value;
    }
    Ok(value as i64)
}

fn decode_polyline_unsigned<I: Iterator<Item = u8>>(bytes: &mut I) -> Result<u64, String> {
    let mut result = 0_u64;
    let mut shift = 0_u32;
    for byte in bytes {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return Err("Invalid journey polyline character".to_string()),
        } as u64;

        result |= (value & 0x1f) << shift;
        if value & 0x20 == 0 {
            return Ok(result);
        }
        shift += 5;
        if shift >= 64 {
            return Err("Invalid journey polyline integer".to_string());
        }
    }
    Err("Truncated journey polyline integer".to_string())
}

fn from_polyline(polyline: &str) -> Result<Gpx, String> {
    validate_polyline_encoding(polyline)?;
    let decoded =
        Polyline::decode(polyline).map_err(|e| format!("Invalid journey polyline: {e}"))?;
    let Polyline::Data3d { coordinates, .. } = decoded else {
        return Err("Journey polyline does not contain elevation data".to_string());
    };
    if coordinates.len() > MAX_GPX_POINTS {
        return Err(format!(
            "Journey contains too many points (maximum {MAX_GPX_POINTS})"
        ));
    }

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

    let mut gpx = Gpx {
        version: GpxVersion::Gpx11,
        ..Default::default()
    };
    gpx.tracks.push(track);
    validate_gpx(&gpx)?;
    Ok(gpx)
}

fn encode(name: &str, polyline: &str) -> String {
    // name and polyline along with the version number are joined with a "|" character and then compressed and base64 encoded
    let version = 1; // version number: change this if the format changes
    let name = name.replace("|", "-"); // replace any "|" characters in the name
    let data = format!("{}|{}|{}", name, version, polyline);
    match zstd::bulk::compress(data.as_bytes(), 0) {
        Ok(compressed) => URL_SAFE_NO_PAD.encode(&compressed),
        Err(e) => {
            log::error!("Error compressing data: {}", e);
            String::new()
        }
    }
}

pub fn decode(encoded: &str) -> Result<Journey, String> {
    if encoded.len() > MAX_JOURNEY_ENCODED_BYTES {
        return Err(format!(
            "Journey code is too large (maximum {MAX_JOURNEY_ENCODED_BYTES} bytes)"
        ));
    }

    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| format!("Invalid journey encoding: {e}"))?;
    if bytes.len() > MAX_JOURNEY_COMPRESSED_BYTES {
        return Err("Compressed journey is too large".to_string());
    }

    let mut decompressor = zstd::bulk::Decompressor::new()
        .map_err(|e| format!("Failed to initialize journey decompressor: {e}"))?;
    let decompressed = decompressor
        .decompress(&bytes, MAX_JOURNEY_DECOMPRESSED_BYTES)
        .map_err(|e| {
            if e.to_string().contains("Destination buffer is too small") {
                format!(
                    "Decompressed journey is too large (maximum {MAX_JOURNEY_DECOMPRESSED_BYTES} bytes)"
                )
            } else {
                format!("Failed to decompress journey: {e}")
            }
        })?;

    let decoded =
        String::from_utf8(decompressed).map_err(|e| format!("Journey is not valid UTF-8: {e}"))?;
    let mut parts = decoded.splitn(3, '|');
    let name = parts
        .next()
        .ok_or_else(|| "Journey name is missing".to_string())?;
    let version = parts
        .next()
        .ok_or_else(|| "Journey version is missing".to_string())?;
    let polyline = parts
        .next()
        .ok_or_else(|| "Journey polyline is missing".to_string())?;

    if version != "1" {
        return Err(format!("Journey version {version} is not supported"));
    }
    if name.len() > MAX_GPX_METADATA_BYTES {
        return Err("Journey name is too large".to_string());
    }

    Ok(Journey {
        name: name.to_string(),
        polyline: polyline.to_string(),
    })
}

pub fn export(name: &str, gpx: &Gpx) -> String {
    let polyline = to_polyline(gpx);
    encode(name, &polyline)
}

pub fn import(journey_string: &str) -> Result<(String, Gpx), String> {
    let journey = decode(journey_string)?;
    let name = journey.name;
    let metadata = gpx::Metadata {
        name: Some(name.clone()),
        ..Default::default()
    };
    let mut gpx = from_polyline(&journey.polyline)?;
    gpx.metadata = Some(metadata);
    validate_gpx(&gpx)?;
    Ok((name, gpx))
}

pub fn import_sample() -> Result<(String, Gpx), String> {
    import(SAMPLE_JOURNEY)
}

fn preflight_gpx<R: Read>(reader: R) -> Result<(), String> {
    use xml::reader::{EventReader, XmlEvent};

    let mut tracks = 0usize;
    let mut routes = 0usize;
    let mut segments = 0usize;
    let mut points = 0usize;
    let mut links = 0usize;
    let mut metadata_bytes = 0usize;
    let mut element_count = 0usize;
    let mut unknown_element_count = 0usize;
    let mut text_elements = Vec::new();

    for event in EventReader::new(reader) {
        match event.map_err(|e| format!("Failed to parse GPX XML: {e}"))? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                increment_limit(&mut element_count, MAX_GPX_XML_ELEMENTS, "XML elements")?;
                if !is_known_gpx_element(&name.local_name) {
                    increment_limit(
                        &mut unknown_element_count,
                        MAX_GPX_UNKNOWN_ELEMENTS,
                        "unknown XML elements",
                    )?;
                }
                match name.local_name.as_str() {
                    "trk" => increment_limit(&mut tracks, MAX_GPX_TRACKS, "tracks")?,
                    "rte" => increment_limit(&mut routes, MAX_GPX_ROUTES, "routes")?,
                    "trkseg" => increment_limit(&mut segments, MAX_GPX_SEGMENTS, "track segments")?,
                    "trkpt" | "rtept" | "wpt" => {
                        increment_limit(&mut points, MAX_GPX_POINTS, "points")?
                    }
                    "link" => increment_limit(&mut links, MAX_GPX_LINKS, "links")?,
                    _ => {}
                }

                for attribute in attributes {
                    if !matches!(
                        attribute.name.local_name.as_str(),
                        "lat" | "lon" | "version"
                    ) {
                        add_string(&mut metadata_bytes, &attribute.value)?;
                    }
                }
                if text_elements.len() >= MAX_GPX_XML_DEPTH {
                    return Err(format!(
                        "GPX XML is nested too deeply (maximum {MAX_GPX_XML_DEPTH} levels)"
                    ));
                }
                text_elements.push(is_metadata_text_element(&name.local_name));
            }
            XmlEvent::EndElement { .. } => {
                text_elements.pop();
            }
            XmlEvent::Characters(text) | XmlEvent::CData(text)
                if text_elements.last().copied().unwrap_or(false) =>
            {
                add_string(&mut metadata_bytes, &text)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn increment_limit(count: &mut usize, maximum: usize, description: &str) -> Result<(), String> {
    *count = count
        .checked_add(1)
        .ok_or_else(|| format!("GPX {description} count overflowed"))?;
    if *count > maximum {
        return Err(format!(
            "GPX contains too many {description} (maximum {maximum})"
        ));
    }
    Ok(())
}

fn is_known_gpx_element(name: &str) -> bool {
    matches!(
        name,
        "gpx"
            | "metadata"
            | "name"
            | "desc"
            | "author"
            | "email"
            | "link"
            | "text"
            | "type"
            | "time"
            | "keywords"
            | "bounds"
            | "copyright"
            | "year"
            | "license"
            | "wpt"
            | "ele"
            | "magvar"
            | "geoidheight"
            | "cmt"
            | "src"
            | "sym"
            | "fix"
            | "sat"
            | "hdop"
            | "vdop"
            | "pdop"
            | "ageofdgpsdata"
            | "dgpsid"
            | "extensions"
            | "rte"
            | "rtept"
            | "number"
            | "trk"
            | "trkseg"
            | "trkpt"
            | "speed"
            | "course"
    )
}

fn is_metadata_text_element(name: &str) -> bool {
    matches!(
        name,
        "name"
            | "cmt"
            | "desc"
            | "src"
            | "type"
            | "sym"
            | "keywords"
            | "author"
            | "email"
            | "text"
            | "fix"
            | "license"
    )
}

fn validate_gpx(gpx: &Gpx) -> Result<(), String> {
    if gpx.tracks.len() > MAX_GPX_TRACKS {
        return Err(format!(
            "GPX contains too many tracks (maximum {MAX_GPX_TRACKS})"
        ));
    }
    if gpx.routes.len() > MAX_GPX_ROUTES {
        return Err(format!(
            "GPX contains too many routes (maximum {MAX_GPX_ROUTES})"
        ));
    }

    let segment_count = gpx
        .tracks
        .iter()
        .try_fold(0usize, |total, track| {
            total.checked_add(track.segments.len())
        })
        .ok_or_else(|| "GPX segment count overflowed".to_string())?;
    if segment_count > MAX_GPX_SEGMENTS {
        return Err(format!(
            "GPX contains too many track segments (maximum {MAX_GPX_SEGMENTS})"
        ));
    }

    let mut point_count = gpx.waypoints.len();
    for track in &gpx.tracks {
        for segment in &track.segments {
            point_count = point_count
                .checked_add(segment.points.len())
                .ok_or_else(|| "GPX point count overflowed".to_string())?;
        }
    }
    for route in &gpx.routes {
        point_count = point_count
            .checked_add(route.points.len())
            .ok_or_else(|| "GPX point count overflowed".to_string())?;
    }
    if point_count > MAX_GPX_POINTS {
        return Err(format!(
            "GPX contains too many points (maximum {MAX_GPX_POINTS})"
        ));
    }

    let mut metadata_bytes = 0usize;
    let mut link_count = 0usize;
    add_optional_string(&mut metadata_bytes, gpx.creator.as_deref())?;
    if let Some(metadata) = &gpx.metadata {
        add_optional_string(&mut metadata_bytes, metadata.name.as_deref())?;
        add_optional_string(&mut metadata_bytes, metadata.description.as_deref())?;
        add_optional_string(&mut metadata_bytes, metadata.keywords.as_deref())?;
        if let Some(author) = &metadata.author {
            add_optional_string(&mut metadata_bytes, author.name.as_deref())?;
            add_optional_string(&mut metadata_bytes, author.email.as_deref())?;
            if let Some(link) = &author.link {
                add_link(&mut metadata_bytes, &mut link_count, link)?;
            }
        }
        if let Some(copyright) = &metadata.copyright {
            add_optional_string(&mut metadata_bytes, copyright.author.as_deref())?;
            add_optional_string(&mut metadata_bytes, copyright.license.as_deref())?;
        }
        for link in &metadata.links {
            add_link(&mut metadata_bytes, &mut link_count, link)?;
        }
    }

    for waypoint in &gpx.waypoints {
        validate_waypoint(waypoint, &mut metadata_bytes, &mut link_count)?;
    }
    for track in &gpx.tracks {
        add_optional_string(&mut metadata_bytes, track.name.as_deref())?;
        add_optional_string(&mut metadata_bytes, track.comment.as_deref())?;
        add_optional_string(&mut metadata_bytes, track.description.as_deref())?;
        add_optional_string(&mut metadata_bytes, track.source.as_deref())?;
        add_optional_string(&mut metadata_bytes, track.type_.as_deref())?;
        for link in &track.links {
            add_link(&mut metadata_bytes, &mut link_count, link)?;
        }
        for segment in &track.segments {
            for waypoint in &segment.points {
                validate_waypoint(waypoint, &mut metadata_bytes, &mut link_count)?;
            }
        }
    }
    for route in &gpx.routes {
        add_optional_string(&mut metadata_bytes, route.name.as_deref())?;
        add_optional_string(&mut metadata_bytes, route.comment.as_deref())?;
        add_optional_string(&mut metadata_bytes, route.description.as_deref())?;
        add_optional_string(&mut metadata_bytes, route.source.as_deref())?;
        add_optional_string(&mut metadata_bytes, route.type_.as_deref())?;
        for link in &route.links {
            add_link(&mut metadata_bytes, &mut link_count, link)?;
        }
        for waypoint in &route.points {
            validate_waypoint(waypoint, &mut metadata_bytes, &mut link_count)?;
        }
    }

    Ok(())
}

fn validate_waypoint(
    waypoint: &Waypoint,
    metadata_bytes: &mut usize,
    link_count: &mut usize,
) -> Result<(), String> {
    let point = waypoint.point();
    if !point.x().is_finite()
        || !point.y().is_finite()
        || !(-180.0..=180.0).contains(&point.x())
        || !(-90.0..=90.0).contains(&point.y())
    {
        return Err("GPX contains an invalid coordinate".to_string());
    }

    add_optional_string(metadata_bytes, waypoint.name.as_deref())?;
    add_optional_string(metadata_bytes, waypoint.comment.as_deref())?;
    add_optional_string(metadata_bytes, waypoint.description.as_deref())?;
    add_optional_string(metadata_bytes, waypoint.source.as_deref())?;
    add_optional_string(metadata_bytes, waypoint.symbol.as_deref())?;
    add_optional_string(metadata_bytes, waypoint.type_.as_deref())?;
    if let Some(gpx::Fix::Other(value)) = &waypoint.fix {
        add_string(metadata_bytes, value)?;
    }
    for link in &waypoint.links {
        add_link(metadata_bytes, link_count, link)?;
    }
    Ok(())
}

fn add_link(metadata_bytes: &mut usize, link_count: &mut usize, link: &Link) -> Result<(), String> {
    *link_count = link_count
        .checked_add(1)
        .ok_or_else(|| "GPX link count overflowed".to_string())?;
    if *link_count > MAX_GPX_LINKS {
        return Err(format!(
            "GPX contains too many links (maximum {MAX_GPX_LINKS})"
        ));
    }
    add_string(metadata_bytes, &link.href)?;
    add_optional_string(metadata_bytes, link.text.as_deref())?;
    add_optional_string(metadata_bytes, link.type_.as_deref())
}

fn add_optional_string(total: &mut usize, value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        add_string(total, value)?;
    }
    Ok(())
}

fn add_string(total: &mut usize, value: &str) -> Result<(), String> {
    *total = total
        .checked_add(value.len())
        .ok_or_else(|| "GPX metadata size overflowed".to_string())?;
    if *total > MAX_GPX_METADATA_BYTES {
        return Err(format!(
            "GPX metadata is too large (maximum {MAX_GPX_METADATA_BYTES} bytes)"
        ));
    }
    Ok(())
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
