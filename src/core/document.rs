use crate::error::ChonkitError;
use docx_rs::read_docx;

/// Load and parse a PDF file from the given buffer.
///
/// * `source`: The buffer.
pub fn load_pdf(source: &[u8]) -> Result<String, ChonkitError> {
    let document = lopdf::Document::load_mem(source)?;
    let text = crate::core::parse::pdf::parse(document)?;
    Ok(text)
}

/// Load and parse a DOCX file from the given buffer.
///
/// * `source`: The buffer.
pub fn load_docx(source: &[u8]) -> Result<String, ChonkitError> {
    let document = read_docx(source)?;
    let text = crate::core::parse::docx::parse(document)?;
    Ok(text)
}
