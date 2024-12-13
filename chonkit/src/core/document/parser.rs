use crate::{core::model::document::DocumentType, error::ChonkitError, map_err};
use docx::DocxParser;
use pdf::PdfParser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use text::TextParser;
use validify::{schema_err, schema_validation, Validate, ValidationErrors};

pub mod docx;
pub mod pdf;
pub mod text;

/// General parsing configuration for documents.
/// A text element is parser specific, it could be PDF pages,
/// DOCX paragraphs, CSV rows, etc.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[validate(Self::validate)]
pub struct ParseConfig {
    /// Skip the first amount of text elements.
    pub start: usize,

    /// Skip the last amount of text elements.
    pub end: usize,

    /// If true, parsers should treat the the (start)[Self::start]
    /// and (end)[Self::end] parameters as a range instead of just
    /// skipping the elements.
    pub range: bool,

    /// Filter specific patterns in text elements. Parser specific.
    #[serde(with = "serde_regex")]
    #[schema(value_type = Vec<String>)]
    pub filters: Vec<Regex>,
}

impl ParseConfig {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            ..Default::default()
        }
    }

    /// Set the parser to use a range of elements instead of just skipping.
    pub fn use_range(mut self) -> Self {
        self.range = true;
        self
    }

    /// Add a filter to the parser.
    /// Each text element (depending on the parser implementation)
    /// will be checked for the regex and will be omitted if it matches.
    ///
    /// * `re`: The expression to match for.
    pub fn with_filter(mut self, re: &str) -> Result<Self, ChonkitError> {
        self.filters.push(map_err!(Regex::new(re)));
        Ok(self)
    }

    #[schema_validation]
    fn validate(&self) -> Result<(), ValidationErrors> {
        if self.range && self.end <= self.start {
            schema_err!(
                "range=true;start>=end",
                "end must be greater than start when using range"
            );
        }
        if self.range && self.start == 0 {
            schema_err!("range=true;start=0", "start cannot be 0 when using range");
        }
    }
}

/// Enumeration of all supported parser types.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Parser {
    Text(TextParser),
    Pdf(PdfParser),
    Docx(DocxParser),
}

impl Parser {
    /// Returns the default parser for a document.
    pub fn new(ty: DocumentType) -> Self {
        match ty {
            // TODO
            DocumentType::Text(_) => Self::Text(TextParser::default()),
            DocumentType::Docx => Self::Docx(DocxParser::default()),
            DocumentType::Pdf => Self::Pdf(PdfParser::default()),
        }
    }

    /// Returns a configured parser for a document.
    pub fn new_from(ty: DocumentType, config: ParseConfig) -> Self {
        match ty {
            // TODO
            DocumentType::Text(_) => Self::Text(TextParser::new(config)),
            DocumentType::Docx => Self::Docx(DocxParser::new(config)),
            DocumentType::Pdf => Self::Pdf(PdfParser::new(config)),
        }
    }

    pub fn parse(&self, input: &[u8]) -> Result<String, ChonkitError> {
        match self {
            Self::Text(p) => p.parse(input),
            Self::Pdf(p) => p.parse(input),
            Self::Docx(p) => p.parse(input),
        }
    }
}
