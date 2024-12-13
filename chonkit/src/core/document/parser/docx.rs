use super::ParseConfig;
use crate::error::ChonkitError;
use crate::map_err;
use docx_rs::read_docx;
use docx_rs::{Paragraph, ParagraphChild, RunChild, Table};
use serde::{Deserialize, Serialize};
use std::{fmt::Write, time::Instant};
use tracing::debug;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DocxParser {
    // TODO: Implement
    config: ParseConfig,
}

impl DocxParser {
    pub fn new(config: ParseConfig) -> Self {
        Self { config }
    }
}

impl DocxParser {
    pub fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        let start = Instant::now();

        let input = map_err!(read_docx(input));
        let mut out = String::new();

        for el in input.document.children {
            match el {
                docx_rs::DocumentChild::Paragraph(ref el) => {
                    let mut paragraph = String::new();
                    let text = map_err!(extract_paragraph(el));
                    for text in text {
                        let text = text.trim();
                        if text.is_empty() {
                            continue;
                        }
                        let _ = write!(paragraph, "{text} ");
                    }
                    let _ = writeln!(out, "{paragraph}");
                }
                docx_rs::DocumentChild::Table(el) => {
                    let table = map_err!(extract_table(*el));
                    let _ = writeln!(out, "{table}");
                }
                _ => {}
            }
        }

        debug!(
            "Finished processing DOCX, took {}ms",
            Instant::now().duration_since(start).as_millis()
        );

        Ok(out)
    }
}

/// Given a DOCX table, create the equivalent table in Markdown style.
///
/// * `table`: The table to process.
fn extract_table(table: Table) -> Result<String, std::fmt::Error> {
    let mut table_out = String::new();

    for row in table.rows.iter() {
        #[allow(irrefutable_let_patterns)]
        let docx_rs::TableChild::TableRow(docx_rs::TableRow { cells, .. }) = row
        else {
            continue;
        };

        let mut row_buf: Vec<String> = vec![];

        for cell in cells.iter() {
            #[allow(irrefutable_let_patterns)]
            let docx_rs::TableRowChild::TableCell(cell) = cell
            else {
                continue;
            };

            let mut cell_buf = String::new();

            for child in cell.children.iter() {
                match child {
                    docx_rs::TableCellContent::Paragraph(ref p) => {
                        let text = extract_paragraph(p)?;
                        write!(cell_buf, " {} ", text.join(""))?;
                    }
                    c => debug!("Unrecognized child in table cell: {:?}", c),
                }
            }

            row_buf.push(cell_buf);
        }

        writeln!(table_out, "|{}|", row_buf.join("|").replace("  ", " "))?;
        write!(table_out, "|")?;

        for cell in row_buf.iter() {
            if cell.is_empty() {
                write!(table_out, "{}|", "-".repeat(cell.len()))?;
                continue;
            }
            write!(table_out, "{}|", "-".repeat(cell.len()))?;
        }

        writeln!(table_out)?;
    }

    Ok(table_out)
}

fn extract_paragraph(p: &Paragraph) -> Result<Vec<&str>, std::fmt::Error> {
    let mut out = vec![];

    for child in p.children.iter() {
        match child {
            docx_rs::ParagraphChild::Run(run) => {
                for rchild in run.children.iter() {
                    let RunChild::Text(t) = rchild else { continue };
                    out.push(t.text.as_str());
                }
            }
            docx_rs::ParagraphChild::Hyperlink(hl) => {
                for rchild in hl.children.iter() {
                    let ParagraphChild::Run(run) = rchild else {
                        continue;
                    };
                    for rchild in run.children.iter() {
                        let RunChild::Text(t) = rchild else { continue };
                        out.push(t.text.as_str());
                    }
                }
            }
            _el => {}
        }
    }

    Ok(out)
}
