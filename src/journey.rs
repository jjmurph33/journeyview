use base64::{Engine, engine::general_purpose::URL_SAFE};
use flexpolyline::Polyline;
use geo_types::Point;
use gpx::{Gpx, GpxVersion, Metadata, Track, TrackSegment, Waypoint};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;
use zstd;

#[derive(Serialize, Deserialize)]
pub struct Journey {
    name: String,
    polyline: String,
}

impl Journey {
    fn new(name: &str, polyline: &str) -> Journey {
        Journey {
            name: name.to_string(),
            polyline: polyline.to_string(),
        }
    }
}

pub fn load_gpx_file(file_path: &str) -> Result<Gpx, String> {
    match fs::File::open(&file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match gpx::read(reader) {
                Ok(gpx) => {
                    //println!("{:?}", gpx);
                    return Ok(gpx);
                }
                Err(e) => {
                    return Err(format!("Failed to parse GPX file: {}", e));
                }
            }
        }
        Err(e) => {
            return Err(format!("Failed to open file: {}", e));
        }
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
    polyline.encode().unwrap_or(String::new())
}

fn from_polyline(polyline: &str) -> Gpx {
    let mut gpx: Gpx = Default::default();
    gpx.version = GpxVersion::Gpx11;
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
    return String::new();
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
                        let journey = Journey::new(name, polyline);
                        return Some(journey);
                    }
                }
                Err(e) => eprintln!("Error decoding imported data: {}", e),
            },
            Err(e) => eprintln!("Error decompressing imported data: {}", e),
        },
        Err(e) => eprintln!("Error reading imported data: {}", e),
    }
    return None;
}

pub fn export(name: &str, gpx: &Gpx) -> String {
    let polyline = to_polyline(&gpx);
    let journey_string = encode(&name, &polyline);
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
        let mut metadata = Metadata::default();
        metadata.name = Some(name.clone());
        let mut gpx = from_polyline(&journey.polyline);
        gpx.metadata = Some(metadata);
        Ok((name, gpx))
    } else {
        Err(String::from("Not a valid Journey"))
    }
}

pub fn name_from_gpx(gpx: &Gpx) -> String {
    if let Some(metadata) = &gpx.metadata {
        if let Some(name) = &metadata.name {
            return name.clone();
        }
    }
    return String::new();
}

pub fn import_sample() -> Result<(String, Gpx), String> {
    import(SAMPLE_JOURNEY)
}

const SAMPLE_JOURNEY: &str = r#"KLUv_QCIVSoAqnUgFCUAScwcULHvkvC37OEHp0Cb4NwIOWhTUlKCi0r5LuL_nf7__y8hPAE5ATYBB9YGKJguZGfEC4uenNkuMPWOqSa5pkykF8Z2L9scOCZh21NHFLGP6eF74wk5soR1aw-5QHS_E6Q3ZaQfwsA-QRvJrUTsN1nI3xZIr0lsncHrn8ZgH7YDFBcGvoVXK8PadmW1dE2tX42t6F20QT44lolxZfXSlP9pEf0aJeBWmIL9V7aAm_QsjezbDv6xnNKt5P4htookyg54F3z2rQO50BSLxxZEr4DPgUTRZcRe1reeFl3gyDLFTeOp1DQL3jSaoJFoMZ6kp5qwYIqpbApfMgmtYVPMLLTSmnZhpwNP8S1qprFvhifJrd3wt2QkRUQp0k-7tSOy2Ucgo6011n9JxPshD2llVxbbJDS1TtFCKiZF5Jhi_5NOkrSphh9BK_KDWpHCyksVHUFALYZZGkk6EMfZkIANFlAuBDBZBAx8jqR9acf4MGJfg4ToE9hEbNjCs8aSiAnbRY-y-c3uo_Chb8Iqt-Gtr11moWSKq0k7xs72tI5lhh-JAHfN4DNp4d2CLwst45DQJ4KhZ2wnDW08e3x8djDgSSTt-GmEGPqDztoPPFlow5Zsik4aW3ncvso-thHa9SPQED1Lm0iKYGQXk04tFQ5I76KG5QI7umHh5B9CQfQMVtIe2EG9MLePHS9tA_5tkiRNWUknoJNPQ0M_S6_8zfspOZEYlDTsMyjJv-C2kAjfkvXUfgUjjQW1ld6Nt8YfmJIakx4OnlmrFRQlRF7Ayj-KUvsYZSJHHPw7ZLMPw-YZwdsvw4VsjDVpm8canfoEfUluapHgUyeCD78RjqQamNrkulkCbjWwwpdUq7HPQ1ubWlP_lBYirPgksbaExI4rywEJZsMaonx09ZTjijXBEBtwK34ouHIIL1S14y20DlyIM08Id5wZS3wvfjBw0lmUh76UrYWiBC3JtHCCgR06zTHyYQeXHTqgIclgQ4C9GMKG6MML4Yqdayj6SXau6BBC73DC7igEn9NI4p7QSh0qwHiOHQypUwaeEDsKtiynL9EHL7h0lB9NPIWDk-hZGO_gSgydS4L-BCP8FW4sPWrhPwn4HwPcnF78TgD1LClRSriAC-FNOWOKFQ9W3I5aLlW5uSfcQk9gwd-J2A0owWcQgG5JBxtaCz6UGPaljTcezDB3i3gCflJ1sMWLK7WngAuuGjOcFW78-cAGOuI4cfMQ8EXAOq45P5aA7NiIvwUjvM8yDx0m6PdGYCWNg0vdM5wZEzAhRvDqTnjLtLwVfUQZ9EUiIfRMqei50XPyoZboH2l4sD84SUVsop-CVvQQPOhPBKBfCWFb9NFH5A6MoD-aJT68crZwFnkiL-1HK_8EXuiQfJSd7aKuJEkA3YwzdycfLnGZYWebKVfNPenZzjv3aiW8kzUgV-uo5bFFK4YwJ9Z5__CE92KJd9YZ-9WO97MTZgNzy9dyDE50FfGjED-HlOdQTvomDNQdy_iOgP-zQL9rb3SL8T_w4G-Q1CHk9DPM2BOaeJ5AA3KiIRFL1vDcQYfbsVykib44HX5wuYWiTSznnqlF-9kM2bGTaK-GvJ8seIoEWvg7xPS5A8gvIfBegpdKz0r4q-zQoZS8r7B6kgM7VYizdEsN75N27A9XIsUjKexcTngtbGG38LUGdygdb8lmCn_sE-njH_eHGlSdrpQtDMGv8KFQCQcYCIB9QSnEmhco4SrOzayF3hpmLBkDEaPHOHHEVepugGDi0ya2QoxZwK0CTJSv0glnNVdhOBCCw18LguTOKA=="#;
