//! Local file reader module for text extraction from PDF and PPTX files.
//!
//! Supports:
//! - **PDF** via `pdf-extract`
//! - **PPTX** (Office Open XML) via `zip` + `quick-xml`
//!
//! The legacy binary `.ppt` format is not supported — only `.pptx`.

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when reading local files.
#[derive(Error, Debug)]
pub enum ReaderError {
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("unsupported file format: {0}. Supported formats: pdf, pptx")]
    UnsupportedFormat(String),
    #[error("failed to extract text from PDF: {0}")]
    PdfError(String),
    #[error("failed to extract text from PPTX: {0}")]
    PptxError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("no text content found in file")]
    NoContent,
}

/// Extracted content from a local file.
#[derive(Debug, Clone)]
pub struct FileContent {
    /// The original file path
    pub path: String,
    /// Document title (derived from filename)
    pub title: Option<String>,
    /// Extracted plain text content
    pub text: String,
}

/// Supported file formats for local extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Portable Document Format
    Pdf,
    /// Office Open XML Presentation
    Pptx,
}

impl FileFormat {
    /// Detect format from file extension.
    ///
    /// Returns `None` for unrecognised extensions.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(Self::Pdf),
            "pptx" => Some(Self::Pptx),
            _ => None,
        }
    }
}

/// Check whether a string looks like a URL (starts with http:// or https://).
pub fn is_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

/// Extract text content from a local file.
///
/// Detects the format from the file extension and delegates to the appropriate
/// extractor.
///
/// # Errors
///
/// Returns [`ReaderError::FileNotFound`] if the path does not exist,
/// [`ReaderError::UnsupportedFormat`] for unknown extensions, or a
/// format-specific error if extraction fails.
pub fn extract_from_file(path: &str) -> Result<FileContent, ReaderError> {
    let file_path = Path::new(path);

    if !file_path.exists() {
        return Err(ReaderError::FileNotFound(path.to_string()));
    }

    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let format = FileFormat::from_extension(extension)
        .ok_or_else(|| ReaderError::UnsupportedFormat(extension.to_string()))?;

    let title = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(['_', '-'], " "));

    let text = match format {
        FileFormat::Pdf => extract_pdf(file_path)?,
        FileFormat::Pptx => extract_pptx(file_path)?,
    };

    if text.trim().is_empty() {
        return Err(ReaderError::NoContent);
    }

    Ok(FileContent {
        path: path.to_string(),
        title,
        text,
    })
}

/// Extract plain text from a PDF file.
fn extract_pdf(path: &Path) -> Result<String, ReaderError> {
    let text = pdf_extract::extract_text(path).map_err(|e| ReaderError::PdfError(e.to_string()))?;

    Ok(clean_extracted_text(&text))
}

/// Extract plain text from a PPTX file.
///
/// PPTX is an Office Open XML format — a ZIP archive containing XML slides
/// at `ppt/slides/slideN.xml`. Text lives inside `<a:t>` elements.
fn extract_pptx(path: &Path) -> Result<String, ReaderError> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ReaderError::PptxError(format!("failed to open PPTX archive: {e}")))?;

    // Collect and sort slide filenames for correct ordering
    let mut slide_names: Vec<String> = archive
        .file_names()
        .filter(|name| name.starts_with("ppt/slides/slide") && name.ends_with(".xml"))
        .map(|s| s.to_string())
        .collect();
    slide_names.sort_by(|a, b| natural_slide_order(a, b));

    let mut slides_text = Vec::new();

    for slide_name in &slide_names {
        let slide_file = archive.by_name(slide_name).map_err(|e| {
            ReaderError::PptxError(format!("failed to read slide {slide_name}: {e}"))
        })?;

        let slide_text = extract_text_from_ooxml(slide_file)?;
        if !slide_text.is_empty() {
            slides_text.push(slide_text);
        }
    }

    Ok(slides_text.join("\n\n"))
}

/// Parse an Office Open XML part and extract all `<a:t>` text runs.
fn extract_text_from_ooxml<R: Read>(reader: R) -> Result<String, ReaderError> {
    let mut xml_reader = Reader::from_reader(BufReader::new(reader));
    let mut buf = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let mut inside_text_run = false;

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"a:t" => {
                inside_text_run = true;
            }
            Ok(Event::Text(ref e)) if inside_text_run => {
                if let Ok(text) = e.xml_content() {
                    let fragment = text.trim().to_string();
                    if !fragment.is_empty() {
                        texts.push(fragment);
                    }
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"a:t" => {
                inside_text_run = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ReaderError::PptxError(format!("XML parse error: {e}")));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(texts.join(" "))
}

/// Sort slide filenames by their numeric index so slide10 comes after slide9.
fn natural_slide_order(a: &str, b: &str) -> std::cmp::Ordering {
    let num_a = extract_slide_number(a);
    let num_b = extract_slide_number(b);
    num_a.cmp(&num_b)
}

/// Extract the numeric portion from a slide filename like `ppt/slides/slide12.xml`.
fn extract_slide_number(name: &str) -> u32 {
    name.trim_start_matches("ppt/slides/slide")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(0)
}

/// Clean extracted text: collapse excessive whitespace, remove blank lines.
fn clean_extracted_text(text: &str) -> String {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url() {
        assert!(is_url("https://example.com"));
        assert!(is_url("http://example.com/page"));
        assert!(!is_url("report.pdf"));
        assert!(!is_url("/home/user/doc.pptx"));
        assert!(!is_url("relative/path.pdf"));
    }

    #[test]
    fn test_file_format_detection() {
        assert_eq!(FileFormat::from_extension("pdf"), Some(FileFormat::Pdf));
        assert_eq!(FileFormat::from_extension("PDF"), Some(FileFormat::Pdf));
        assert_eq!(FileFormat::from_extension("pptx"), Some(FileFormat::Pptx));
        assert_eq!(FileFormat::from_extension("PPTX"), Some(FileFormat::Pptx));
        assert_eq!(FileFormat::from_extension("doc"), None);
        assert_eq!(FileFormat::from_extension("ppt"), None);
    }

    #[test]
    fn test_natural_slide_order() {
        let mut names = vec![
            "ppt/slides/slide10.xml".to_string(),
            "ppt/slides/slide2.xml".to_string(),
            "ppt/slides/slide1.xml".to_string(),
        ];
        names.sort_by(|a, b| natural_slide_order(a, b));
        assert_eq!(
            names,
            vec![
                "ppt/slides/slide1.xml",
                "ppt/slides/slide2.xml",
                "ppt/slides/slide10.xml",
            ]
        );
    }

    #[test]
    fn test_extract_slide_number() {
        assert_eq!(extract_slide_number("ppt/slides/slide1.xml"), 1);
        assert_eq!(extract_slide_number("ppt/slides/slide12.xml"), 12);
        assert_eq!(extract_slide_number("ppt/slides/slideXYZ.xml"), 0);
    }

    #[test]
    fn test_clean_extracted_text() {
        let input = "  hello   world  \n\n  foo   bar  \n\n\n  baz  ";
        let result = clean_extracted_text(input);
        assert_eq!(result, "hello world\nfoo bar\nbaz");
    }

    #[test]
    fn test_file_not_found() {
        let result = extract_from_file("/nonexistent/file.pdf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReaderError::FileNotFound(_)));
    }

    #[test]
    fn test_unsupported_format() {
        // Create a temp file with an unsupported extension
        let tmp = tempfile::Builder::new()
            .suffix(".docx")
            .tempfile()
            .expect("failed to create temp file");
        let path = tmp.path().to_str().expect("path should be valid UTF-8");

        let result = extract_from_file(path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReaderError::UnsupportedFormat(_)
        ));
    }
}
