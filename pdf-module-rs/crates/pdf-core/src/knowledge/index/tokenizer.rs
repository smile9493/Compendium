//! CJK-aware tokenizer for Tantivy full-text search.
//!
//! Uses `jieba-rs` for proper Chinese word segmentation, replacing the old
//! character n-gram approach that produced noisy fragments.
//!
//! Registered as `"cjk"` — use this name in field options.

use jieba_rs::Jieba;
use std::sync::LazyLock;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// Global Jieba instance (lazy-initialized, thread-safe).
/// Jieba's dictionary loading is expensive — do it once.
static JIEBA: LazyLock<Jieba> = LazyLock::new(Jieba::new);

/// A Tantivy-compatible tokenizer backed by jieba-rs.
#[derive(Clone, Debug)]
pub struct JiebaTokenizer;

/// The token stream produced by `JiebaTokenizer`.
pub struct JiebaTokenStream {
    tokens: Vec<Token>,
    offset: usize,
}

impl Tokenizer for JiebaTokenizer {
    type TokenStream<'a> = JiebaTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let words = JIEBA.cut(text, false);
        let mut tokens = Vec::with_capacity(words.len());
        let mut offset = 0usize;

        for word in words {
            let trimmed = word.trim();
            if trimmed.is_empty() {
                offset += word.len();
                continue;
            }
            let start = offset;
            let end = offset + word.len();
            tokens.push(Token {
                offset_from: start,
                offset_to: end,
                position: tokens.len(),
                text: trimmed.to_string(),
                position_length: 1,
            });
            offset = end;
        }

        JiebaTokenStream { tokens, offset: 0 }
    }
}

impl TokenStream for JiebaTokenStream {
    fn advance(&mut self) -> bool {
        if self.offset < self.tokens.len() {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.tokens[self.offset - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.offset - 1]
    }
}

/// Register the CJK jieba tokenizer on a Tantivy index.
///
/// Registers as `"cjk"` — use this name in schema field options.
/// The pipeline applies jieba segmentation followed by lowercasing
/// for case-insensitive English token matching.
pub fn register_cjk_tokenizer(index: &tantivy::Index) {
    use tantivy::tokenizer::{LowerCaser, TextAnalyzer};
    let analyzer = TextAnalyzer::builder(JiebaTokenizer).filter(LowerCaser).build();
    index.tokenizers().register("cjk", analyzer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jieba_segmentation() {
        // Use the full pipeline via TextAnalyzer to include LowerCaser
        use tantivy::tokenizer::{LowerCaser, TextAnalyzer};
        let mut analyzer = TextAnalyzer::builder(JiebaTokenizer).filter(LowerCaser).build();
        let mut stream = analyzer.token_stream("从 Transformer 到大规模语言模型的演进");
        let mut words = Vec::new();
        while stream.advance() {
            words.push(stream.token().text.clone());
        }
        // Should produce meaningful word-level tokens (lowercased)
        assert!(words.contains(&"transformer".to_string()));
        assert!(words.contains(&"大规模".to_string()) || words.contains(&"大".to_string()));
        assert!(words.contains(&"语言".to_string()));
        assert!(words.contains(&"模型".to_string()));
        // Should NOT produce noise like "的文", "件中"
        assert!(!words.contains(&"从 transformer".to_string()));
    }

    #[test]
    fn test_raw_tokenizer_preserves_case() {
        let mut tokenizer = JiebaTokenizer;
        let mut stream = tokenizer.token_stream("从 Transformer 到大规模语言模型的演进");
        let mut words = Vec::new();
        while stream.advance() {
            words.push(stream.token().text.clone());
        }
        // Raw JiebaTokenizer preserves original case
        assert!(words.contains(&"Transformer".to_string()));
    }

    #[test]
    fn test_no_empty_tokens() {
        let mut tokenizer = JiebaTokenizer;
        let mut stream = tokenizer.token_stream("  hello  world  ");
        let mut words = Vec::new();
        while stream.advance() {
            let tok = stream.token();
            assert!(!tok.text.trim().is_empty(), "should not produce empty tokens");
            words.push(tok.text.clone());
        }
        assert!(!words.is_empty());
    }
}
