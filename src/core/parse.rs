pub mod docx;
pub mod pdf;

pub type Parser<T> = fn(T) -> String;
