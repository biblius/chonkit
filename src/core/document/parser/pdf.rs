use super::{DocumentParser, ParseConfig};
use crate::{core::model::document::DocumentType, error::ChonkitError};
use lopdf::Object;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Write,
    io::{Error, ErrorKind},
    time::Instant,
};
use tracing::debug;

/// Parses PDFs.
/// Configuration:
/// * `skip_front`: The amount of PDF pages to skip from the start of the document.
/// * `skip_back`: The amount of pages to omit from the back of the document.
/// * `filters`: Line based, i.e. lines matching a filter will be skipped.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PdfParser {
    config: ParseConfig,
}

impl PdfParser {
    pub fn new(config: ParseConfig) -> Self {
        Self { config }
    }
}

impl DocumentParser for PdfParser {
    fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        let _start = Instant::now();

        let ParseConfig {
            skip_front,
            skip_back,
            ref filters,
        } = self.config;

        let mut input = lopdf::Document::load_mem(input)?;

        // Filter unwanted objects.
        input.objects.retain(filter_object);

        let mut out = String::new();

        let pages = input.page_iter();

        // Size hint of pages is the amount
        let total_pages = pages.size_hint().0;

        for (page_num, page_id) in pages
            .enumerate()
            .map(|(page_num, oid)| (page_num as u32 + 1, oid))
            .skip(skip_front as usize)
        {
            if total_pages - (page_num as usize - 1) - skip_back as usize == 0 {
                break;
            }

            let text = input.extract_text(&[page_num]).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to extract text from page {page_num} id={page_id:?}: {e:?}"),
                )
            })?;

            'lines: for line in text.lines() {
                let line = line.trim();

                // Skip lines numbers in output.
                if line == page_num.to_string() {
                    continue;
                }

                for filter in filters.iter() {
                    if filter.is_match(line) {
                        continue 'lines;
                    }
                }

                let _ = writeln!(out, "{line}");
            }
        }

        debug!(
            "Finished processing PDF, took {}ms",
            Instant::now().duration_since(_start).as_millis()
        );

        Ok(out)
    }

    fn dtype(&self) -> DocumentType {
        DocumentType::Pdf
    }
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

/// Filters unwanted properties in an object and
/// returns whether to keep it or not.
///
/// * `object`: PDF object.
fn filter_object(_: &(u32, u16), object: &mut Object) -> bool {
    if IGNORE.contains(&object.type_name().unwrap_or_default()) {
        return false;
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
            return false;
        }
    }

    true
}
