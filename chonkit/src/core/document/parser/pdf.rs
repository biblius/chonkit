use super::ParseConfig;
use crate::{err, error::ChonkitError, map_err};
use pdfium_render::prelude::Pdfium;
use serde::{Deserialize, Serialize};
use std::{fmt::Write, time::Instant};
use tracing::debug;

/// Parses PDFs.
/// Configuration:
/// * `skip_start`: The amount of PDF pages to skip from the start of the document.
/// * `skip_end`: The amount of pages to omit from the back of the document.
/// * `range`: Range of pages to use.
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

impl PdfParser {
    pub fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        let _start = Instant::now();

        let ParseConfig {
            start,
            end,
            ref filters,
            range,
        } = self.config;

        let pdfium = Pdfium::default();
        let input = map_err!(pdfium.load_pdf_from_byte_slice(input, None));

        let mut out = String::new();

        let pages = input.pages();

        let total_pages = pages.len();

        let start = if range { start - 1 } else { start };
        let end_condition: Box<dyn Fn(usize) -> bool> = if range {
            Box::new(|page_num| page_num == end.saturating_sub(1))
        } else {
            Box::new(|page_num| {
                total_pages
                    .saturating_sub(page_num as u16)
                    .saturating_sub(end as u16)
                    == 0
            })
        };

        // For debugging
        let mut page_count = 0;

        for (page_num, page) in pages.iter().enumerate().skip(start) {
            if end_condition(page_num) {
                break;
            }

            // page_num is 0 based
            let text = map_err!(page.text());

            'lines: for line in text.all().lines() {
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

            page_count += 1;
        }

        debug!(
            "Finished processing PDF, {page_count}/{total_pages} pages took {}ms",
            Instant::now().duration_since(_start).as_millis()
        );

        if out.is_empty() {
            tracing::error!(
                "Parsing resulted in empty output. Config: {:?}",
                self.config
            );

            return err!(
                ParseConfig,
                "empty output (total pages: {total_pages} | start: {start} | end: {end} | range: {range})",
            );
        }

        Ok(out)
    }
}
