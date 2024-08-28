use std::{
    fmt::Write,
    io::{Error, ErrorKind},
    time::Instant,
};

use lopdf::Object;
use tracing::{debug, warn};

pub fn parse(input: lopdf::Document) -> Result<String, Error> {
    let start = Instant::now();

    let mut text = String::new();

    let pages: Vec<Result<String, Error>> = input
        .page_iter()
        .map(|(page_num, page_id)| -> Result<String, Error> {
            let text = input.extract_text(&[page_num]).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to extract text from page {page_num} id={page_id:?}: {e:?}"),
                )
            })?;
            Ok(text
                .split('\n')
                .map(|s| s.trim_end().to_string())
                .collect::<String>())
        })
        .collect();

    for page in pages {
        match page {
            Ok(content) => {
                let _ = writeln!(text, "{content}");
            }
            Err(e) => warn!("{e}"),
        }
    }

    debug!(
        "Finished processing PDF, took {}ms",
        Instant::now().duration_since(start).as_millis()
    );

    Ok(text)
}

static IGNORE: &[&str] = &[
    "Length",
    "BBox",
    "FormType",
    "Matrix",
    "Type",
    "XObject",
    "Subtype",
    "Filter",
    "ColorSpace",
    "Width",
    "Height",
    "BitsPerComponent",
    "Length1",
    "Length2",
    "Length3",
    "PTEX.FileName",
    "PTEX.PageNumber",
    "PTEX.InfoDict",
    "FontDescriptor",
    "ExtGState",
    "MediaBox",
    "Annot",
];

fn filter_func(object_id: (u32, u16), object: &mut Object) -> Option<((u32, u16), Object)> {
    if IGNORE.contains(&object.type_name().unwrap_or_default()) {
        return None;
    }
    if let Ok(d) = object.as_dict_mut() {
        d.remove(b"Producer");
        d.remove(b"ModDate");
        d.remove(b"Creator");
        d.remove(b"ProcSet");
        d.remove(b"XObject");
        d.remove(b"MediaBox");
        d.remove(b"Annots");
        if d.is_empty() {
            return None;
        }
    }
    Some((object_id, object.to_owned()))
}
