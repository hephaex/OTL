//! OTL OCR - Optical Character Recognition integration
//!
//! Provides OCR capabilities for scanned documents using
//! Tesseract or PaddleOCR backends.

use std::path::Path;
use std::process::Command;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OcrError {
    #[error("OCR engine not available: {0}")]
    EngineNotAvailable(String),

    #[error("Image processing failed: {0}")]
    ImageProcessingFailed(String),

    #[error("OCR execution failed: {0}")]
    ExecutionFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, OcrError>;

/// OCR result for a single page
#[derive(Debug, Clone)]
pub struct OcrResult {
    /// Extracted text content
    pub text: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Page number
    pub page: u32,
    /// Detected language
    pub language: Option<String>,
}

impl OcrResult {
    /// Create a new OCR result
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            confidence: 1.0,
            page: 1,
            language: None,
        }
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set page number
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Set detected language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

/// Trait for OCR engines
pub trait OcrEngine: Send + Sync {
    /// Extract text from an image file
    fn extract_text(&self, image_path: &Path) -> Result<OcrResult>;

    /// Extract text from multiple images (e.g., PDF pages)
    fn extract_text_batch(&self, image_paths: &[&Path]) -> Result<Vec<OcrResult>> {
        image_paths
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let mut result = self.extract_text(path)?;
                result.page = (i + 1) as u32;
                Ok(result)
            })
            .collect()
    }

    /// Check if the engine is available on the system
    fn is_available(&self) -> bool;

    /// Get the engine name
    fn name(&self) -> &str;
}

// ============================================================================
// Tesseract OCR Engine
// ============================================================================

/// Tesseract OCR engine configuration
#[derive(Debug, Clone)]
pub struct TesseractConfig {
    /// Language code(s) for OCR (e.g., "eng", "kor", "eng+kor")
    pub language: String,
    /// Page segmentation mode (PSM)
    pub psm: Option<u8>,
    /// OCR engine mode (OEM)
    pub oem: Option<u8>,
    /// Path to tesseract executable
    pub executable_path: Option<String>,
    /// Additional tesseract arguments
    pub extra_args: Vec<String>,
}

impl Default for TesseractConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            psm: None,
            oem: None,
            executable_path: None,
            extra_args: Vec::new(),
        }
    }
}

impl TesseractConfig {
    /// Create config for Korean + English OCR
    pub fn korean() -> Self {
        Self {
            language: "kor+eng".to_string(),
            ..Default::default()
        }
    }

    /// Set language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Set page segmentation mode
    pub fn with_psm(mut self, psm: u8) -> Self {
        self.psm = Some(psm);
        self
    }

    /// Set OCR engine mode
    pub fn with_oem(mut self, oem: u8) -> Self {
        self.oem = Some(oem);
        self
    }
}

/// Tesseract OCR engine wrapper
pub struct TesseractEngine {
    config: TesseractConfig,
}

impl TesseractEngine {
    /// Create a new Tesseract engine with default config
    pub fn new() -> Self {
        Self {
            config: TesseractConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: TesseractConfig) -> Self {
        Self { config }
    }

    /// Get the tesseract executable path
    fn executable(&self) -> &str {
        self.config
            .executable_path
            .as_deref()
            .unwrap_or("tesseract")
    }

    /// Build command arguments
    fn build_args(&self, image_path: &Path) -> Vec<String> {
        let mut args = vec![
            image_path.display().to_string(),
            "stdout".to_string(), // Output to stdout
            "-l".to_string(),
            self.config.language.clone(),
        ];

        if let Some(psm) = self.config.psm {
            args.push("--psm".to_string());
            args.push(psm.to_string());
        }

        if let Some(oem) = self.config.oem {
            args.push("--oem".to_string());
            args.push(oem.to_string());
        }

        args.extend(self.config.extra_args.clone());
        args
    }

    /// Parse confidence from tesseract hOCR output (if available)
    fn parse_confidence(&self, _output: &str) -> f32 {
        // Default confidence when not using hOCR
        // Could be enhanced to use hOCR output for actual confidence
        0.9
    }
}

impl Default for TesseractEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrEngine for TesseractEngine {
    fn extract_text(&self, image_path: &Path) -> Result<OcrResult> {
        if !self.is_available() {
            return Err(OcrError::EngineNotAvailable(
                "Tesseract is not installed or not in PATH".to_string(),
            ));
        }

        let args = self.build_args(image_path);

        let output = Command::new(self.executable())
            .args(&args)
            .output()
            .map_err(|e| OcrError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OcrError::ExecutionFailed(format!(
                "Tesseract failed: {stderr}"
            )));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let confidence = self.parse_confidence(&text);

        Ok(OcrResult {
            text,
            confidence,
            page: 1,
            language: Some(self.config.language.clone()),
        })
    }

    fn is_available(&self) -> bool {
        Command::new(self.executable())
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn name(&self) -> &str {
        "tesseract"
    }
}

// ============================================================================
// OCR Manager
// ============================================================================

/// OCR manager that handles multiple engines
pub struct OcrManager {
    engines: Vec<Box<dyn OcrEngine>>,
}

impl OcrManager {
    /// Create a new OCR manager with default engines
    pub fn new() -> Self {
        let mut manager = Self {
            engines: Vec::new(),
        };

        // Register Tesseract by default
        let tesseract = TesseractEngine::new();
        if tesseract.is_available() {
            manager.register(tesseract);
        }

        manager
    }

    /// Register an OCR engine
    pub fn register<E: OcrEngine + 'static>(&mut self, engine: E) {
        self.engines.push(Box::new(engine));
    }

    /// Check if any OCR engine is available
    pub fn is_available(&self) -> bool {
        !self.engines.is_empty()
    }

    /// Get available engines
    pub fn available_engines(&self) -> Vec<&str> {
        self.engines.iter().map(|e| e.name()).collect()
    }

    /// Extract text using the first available engine
    pub fn extract_text(&self, image_path: &Path) -> Result<OcrResult> {
        if self.engines.is_empty() {
            return Err(OcrError::EngineNotAvailable(
                "No OCR engines available".to_string(),
            ));
        }

        // Use first available engine
        self.engines[0].extract_text(image_path)
    }

    /// Extract text from multiple images
    pub fn extract_text_batch(&self, image_paths: &[&Path]) -> Result<Vec<OcrResult>> {
        if self.engines.is_empty() {
            return Err(OcrError::EngineNotAvailable(
                "No OCR engines available".to_string(),
            ));
        }

        self.engines[0].extract_text_batch(image_paths)
    }
}

impl Default for OcrManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_result_builder() {
        let result = OcrResult::new("Hello World")
            .with_confidence(0.95)
            .with_page(2)
            .with_language("eng");

        assert_eq!(result.text, "Hello World");
        assert_eq!(result.confidence, 0.95);
        assert_eq!(result.page, 2);
        assert_eq!(result.language, Some("eng".to_string()));
    }

    #[test]
    fn test_tesseract_config() {
        let config = TesseractConfig::default();
        assert_eq!(config.language, "eng");

        let korean_config = TesseractConfig::korean();
        assert_eq!(korean_config.language, "kor+eng");

        let custom = TesseractConfig::default()
            .with_language("jpn")
            .with_psm(6)
            .with_oem(1);

        assert_eq!(custom.language, "jpn");
        assert_eq!(custom.psm, Some(6));
        assert_eq!(custom.oem, Some(1));
    }

    #[test]
    fn test_tesseract_engine_creation() {
        let engine = TesseractEngine::new();
        assert_eq!(engine.name(), "tesseract");

        let engine_with_config = TesseractEngine::with_config(TesseractConfig::korean());
        assert_eq!(engine_with_config.config.language, "kor+eng");
    }

    #[test]
    fn test_ocr_manager() {
        let manager = OcrManager::new();

        // Note: This test may pass or fail depending on whether
        // Tesseract is installed on the system
        let _ = manager.is_available();
    }
}
