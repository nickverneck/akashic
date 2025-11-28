use super::Extractor;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::Path;

/// PDF Extractor
pub struct PdfExtractor;

#[async_trait]
impl Extractor for PdfExtractor {
    async fn extract(&self, file_path: &str) -> Result<String> {
        let path = Path::new(file_path);
        
        // Try native PDF extraction first
        match pdf_extract::extract_text(path) {
            Ok(text) => Ok(text),
            Err(_) => {
                // Fallback to OCR
                tracing::warn!("PDF extraction failed, falling back to OCR for {}", file_path);
                ocr_fallback(file_path).await
            }
        }
    }

    fn supports(&self, file_path: &str) -> bool {
        file_path.to_lowercase().ends_with(".pdf")
    }
}

/// Markdown Extractor
pub struct MarkdownExtractor;

#[async_trait]
impl Extractor for MarkdownExtractor {
    async fn extract(&self, file_path: &str) -> Result<String> {
        tokio::fs::read_to_string(file_path)
            .await
            .context("Failed to read markdown file")
    }

    fn supports(&self, file_path: &str) -> bool {
        let lower = file_path.to_lowercase();
        lower.ends_with(".md") || lower.ends_with(".markdown")
    }
}

/// Text Extractor
pub struct TextExtractor;

#[async_trait]
impl Extractor for TextExtractor {
    async fn extract(&self, file_path: &str) -> Result<String> {
        tokio::fs::read_to_string(file_path)
            .await
            .context("Failed to read text file")
    }

    fn supports(&self, file_path: &str) -> bool {
        file_path.to_lowercase().ends_with(".txt")
    }
}

/// EPUB Extractor
pub struct EpubExtractor;

#[async_trait]
impl Extractor for EpubExtractor {
    async fn extract(&self, file_path: &str) -> Result<String> {
        let doc = epub::doc::EpubDoc::new(file_path)
            .context("Failed to open EPUB file")?;
        
        let mut text = String::new();
        let spine_len = doc.spine.len();
        
        for i in 0..spine_len {
            let mut doc = epub::doc::EpubDoc::new(file_path)?;
            doc.set_current_chapter(i);
            
            if let Some((content, _)) = doc.get_current_str() {
                // Strip HTML tags (basic approach)
                let stripped = strip_html_tags(&content);
                text.push_str(&stripped);
                text.push('\n');
            }
        }
        
        Ok(text)
    }

    fn supports(&self, file_path: &str) -> bool {
        file_path.to_lowercase().ends_with(".epub")
    }
}

/// DOC/DOCX Extractor (placeholder - requires additional dependencies)
pub struct DocExtractor;

#[async_trait]
impl Extractor for DocExtractor {
    async fn extract(&self, file_path: &str) -> Result<String> {
        // For now, we'll use OCR as fallback for DOC files
        // In production, you might want to use a library like docx-rs or call external tools
        tracing::warn!("DOC extraction not fully implemented, using OCR for {}", file_path);
        ocr_fallback(file_path).await
    }

    fn supports(&self, file_path: &str) -> bool {
        let lower = file_path.to_lowercase();
        lower.ends_with(".doc") || lower.ends_with(".docx")
    }
}

/// OCR fallback using Tesseract
async fn ocr_fallback(file_path: &str) -> Result<String> {
    // Use tesseract CLI
    let output = tokio::process::Command::new("tesseract")
        .arg(file_path)
        .arg("stdout")
        .output()
        .await
        .context("Failed to run tesseract. Make sure it's installed.")?;

    if !output.status.success() {
        anyhow::bail!(
            "Tesseract failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8(output.stdout).context("Invalid UTF-8 from tesseract")
}

/// Strip HTML tags (basic implementation)
fn strip_html_tags(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(html, "").to_string()
}

/// Factory to get the appropriate extractor
pub fn get_extractor(file_path: &str) -> Option<Box<dyn Extractor>> {
    let extractors: Vec<Box<dyn Extractor>> = vec![
        Box::new(PdfExtractor),
        Box::new(MarkdownExtractor),
        Box::new(TextExtractor),
        Box::new(EpubExtractor),
        Box::new(DocExtractor),
    ];

    extractors
        .into_iter()
        .find(|e| e.supports(file_path))
}
