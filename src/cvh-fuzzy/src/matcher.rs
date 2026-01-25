//! Custom matcher utilities
//!
//! Wraps nucleo for fuzzy matching with additional features

use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};

/// Score threshold for filtering results
pub const MIN_SCORE: u32 = 1;

/// Wrapper around nucleo matcher
pub struct FuzzyMatcher {
    matcher: Matcher,
    config: Config,
}

impl FuzzyMatcher {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
            config: Config::DEFAULT,
        }
    }

    /// Match a pattern against a haystack, returning score if matched
    pub fn score(&mut self, pattern: &str, haystack: &str) -> Option<u32> {
        if pattern.is_empty() {
            return Some(0);
        }

        let pattern = Pattern::parse(pattern, CaseMatching::Smart, Normalization::Smart);
        let mut buf = vec![];
        let haystack = Utf32Str::new(haystack, &mut buf);

        pattern.score(haystack, &mut self.matcher)
    }

    /// Check if pattern matches haystack
    pub fn matches(&mut self, pattern: &str, haystack: &str) -> bool {
        self.score(pattern, haystack).is_some()
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pattern() {
        let mut matcher = FuzzyMatcher::new();
        assert!(matcher.matches("", "anything"));
    }

    #[test]
    fn test_exact_match() {
        let mut matcher = FuzzyMatcher::new();
        assert!(matcher.matches("firefox", "Firefox"));
    }

    #[test]
    fn test_fuzzy_match() {
        let mut matcher = FuzzyMatcher::new();
        assert!(matcher.matches("ff", "Firefox"));
        assert!(matcher.matches("frfx", "Firefox"));
    }

    #[test]
    fn test_no_match() {
        let mut matcher = FuzzyMatcher::new();
        assert!(!matcher.matches("xyz", "Firefox"));
    }
}
