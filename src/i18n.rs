//! Internationalization (i18n) support
//!
//! This module handles locale detection and initialization for multi-language support.
//! Supported languages: English (en), Korean (ko), Japanese (ja)
//!
//! The locale is detected from the LANG environment variable.

use rust_i18n::set_locale;
use sys_locale::get_locale;

/// Supported locales
const SUPPORTED_LOCALES: &[&str] = &["en", "ko", "ja"];

/// Default locale when system locale is not supported
const DEFAULT_LOCALE: &str = "en";

/// Initialize the locale based on system settings (LANG environment variable)
/// 
/// Detects the system locale and sets the appropriate language.
/// Falls back to English if the system locale is not supported.
pub fn init_locale() {
    let locale = detect_locale();
    set_locale(&locale);
}

/// Detect the system locale and return a supported locale code
fn detect_locale() -> String {
    if let Some(locale) = get_locale() {
        // Extract language code (e.g., "en-US" -> "en", "ko-KR" -> "ko")
        let lang = locale.split(&['-', '_'][..]).next().unwrap_or(DEFAULT_LOCALE);
        
        if SUPPORTED_LOCALES.contains(&lang) {
            return lang.to_string();
        }
    }
    
    DEFAULT_LOCALE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_locale() {
        let locale = detect_locale();
        assert!(SUPPORTED_LOCALES.contains(&locale.as_str()) || locale == DEFAULT_LOCALE);
    }
}
