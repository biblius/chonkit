use crate::{core::model::document::DocumentType, error::ChonkitError};
use docx::DocxParser;
use pdf::PdfParser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use text::TextParser;

pub mod docx;
pub mod pdf;
pub mod text;

/// Implement on anything that has to parse document bytes.
pub trait DocumentParser {
    fn dtype(&self) -> DocumentType;

    fn parse(&self, input: &[u8]) -> Result<String, ChonkitError>;
}

/// General parsing configuration for documents.
/// A text element is parser specific, it could be PDF pages,
/// DOCX paragraphs, CSV rows, etc.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ParseConfig {
    /// Skip the first amount of text elements.
    pub skip_start: usize,

    /// Skip the last amount of text elements.
    pub skip_end: usize,

    /// If given, parsers should prioritize over the skips.
    /// Set the range of text elements to be parsed.
    pub range: Option<(usize, usize)>,

    /// Filter specific patterns in text elements. Parser specific.
    #[serde(with = "serde_regex")]
    pub filters: Vec<Regex>,
}

impl ParseConfig {
    pub fn new(skip_start: usize, skip_end: usize) -> Self {
        Self {
            skip_start,
            skip_end,
            ..Default::default()
        }
    }

    /// Set the range of elements to parse to parse.
    pub fn use_range(mut self, start: usize, end: usize) -> Self {
        self.range = Some((start, end));
        self
    }

    /// Add a filter to the parser.
    /// Each text element (depending on the parser implementation)
    /// will be checked for the regex
    /// and will be omitted if it matches.
    ///
    /// * `re`: The expression to match for.
    pub fn filter(mut self, re: Regex) -> Self {
        self.filters.push(re);
        self
    }
}

/// Enumeration of all supported parser types.
#[derive(Debug, Serialize, Deserialize)]
pub enum Parser {
    Text(TextParser),
    Pdf(PdfParser),
    Docx(DocxParser),
}

impl Parser {
    /// Returns the default parser for a document.
    pub fn new(ty: DocumentType) -> Self {
        match ty {
            DocumentType::Text => Self::Text(TextParser::default()),
            DocumentType::Docx => Self::Docx(DocxParser::default()),
            DocumentType::Pdf => Self::Pdf(PdfParser::default()),
        }
    }

    /// Returns a configured parser for a document.
    pub fn new_from(ty: DocumentType, config: ParseConfig) -> Self {
        match ty {
            DocumentType::Text => Self::Text(TextParser::new(config)),
            DocumentType::Docx => Self::Docx(DocxParser::new(config)),
            DocumentType::Pdf => Self::Pdf(PdfParser::new(config)),
        }
    }
}

impl DocumentParser for Parser {
    fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        match self {
            Self::Text(p) => p.parse(input),
            Self::Pdf(p) => p.parse(input),
            Self::Docx(p) => p.parse(input),
        }
    }

    fn dtype(&self) -> DocumentType {
        match self {
            Self::Text(p) => p.dtype(),
            Self::Pdf(p) => p.dtype(),
            Self::Docx(p) => p.dtype(),
        }
    }
}
