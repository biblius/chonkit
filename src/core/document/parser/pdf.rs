use super::DocumentParser;
use crate::error::ChonkitError;
use lopdf::Object;
use regex::Regex;
use std::{
    fmt::Write,
    io::{Error, ErrorKind},
    time::Instant,
};
use tracing::debug;

#[derive(Debug, Default)]
/// Parses PDF
///
/// * `skip`:
/// * `line_filters`:
pub struct PdfParser {
    skip: Option<u32>,
    line_filters: Vec<Regex>,
}

impl PdfParser {
    /// Skip the first `mat pages when parsing.
    ///
    /// * `amt`: Amount.
    pub fn skip_pages(mut self, amt: u32) -> Self {
        self.skip = Some(amt);
        self
    }

    /// Add a line filter to the parser.
    /// Each line will be checked for the regex
    /// and will be omitted if it matches.
    ///
    /// * `re`: The expression to match for.
    pub fn line_filter(mut self, re: Regex) -> Self {
        self.line_filters.push(re);
        self
    }
}

impl DocumentParser for PdfParser {
    fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        let _start = Instant::now();

        let mut input = lopdf::Document::load_mem(input)?;

        // Filter unwanted objects.
        input.objects.retain(filter_object);

        let mut out = String::new();

        for (page_num, page_id) in input.get_pages().into_iter() {
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

                for filter in self.line_filters.iter() {
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
