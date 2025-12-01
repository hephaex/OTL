//! OTL OCR - Optical Character Recognition integration
//!
//! Provides OCR capabilities for scanned documents using
//! Tesseract or PaddleOCR backends.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OcrError {
    #[error("OCR engine not available: {0}")]
    EngineNotAvailable(String),

    #[error("Image processing failed: {0}")]
    ImageProcessingFailed(String),

    #[error("OCR execution failed: {0}")]
    ExecutionFailed(String),
}

pub type Result<T> = std::result::Result<T, OcrError>;

/// OCR result for a single page
#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub page: u32,
}

/// Trait for OCR engines
pub trait OcrEngine: Send + Sync {
    /// Extract text from an image file
    fn extract_text(&self, image_path: &std::path::Path) -> Result<OcrResult>;

    /// Check if the engine is available
    fn is_available(&self) -> bool;
}
