use crate::error::ChonkitError;

pub mod docx;
pub mod pdf;

/// Implement on anything that has to parse document bytes.
pub trait DocumentParser {
    fn parse(&self, input: &[u8]) -> Result<String, ChonkitError>;
}

/// Basic parser implementation that reads the input to a string.
pub fn parse_text(input: &[u8]) -> Result<String, ChonkitError> {
    Ok(String::from_utf8_lossy(input).to_string())
}

/// Any function with signature T can be a parser.
impl<T> DocumentParser for T
where
    T: Fn(&[u8]) -> Result<String, ChonkitError>,
{
    fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        self(input)
    }
}
