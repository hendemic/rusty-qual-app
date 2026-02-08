use crate::domain::{FileId, TextBlock};
use unicode_segmentation::UnicodeSegmentation;

/// Splits file content into `TextBlock`s using a cascade strategy:
/// 1. Split on double newlines (paragraphs)
/// 2. If only one result, fall back to single newlines (lines)
/// 3. If still only one result, fall back to sentence boundaries
pub fn split_into_blocks(file_id: FileId, content: &str) -> Vec<TextBlock> {
    if content.is_empty() {
        return Vec::new();
    }

    // Try paragraph splitting first
    let parts: Vec<&str> = content
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() > 1 {
        return parts
            .into_iter()
            .enumerate()
            .map(|(i, text)| TextBlock::new(file_id, i, text.to_string()))
            .collect();
    }

    // Fall back to line splitting
    let parts: Vec<&str> = content
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() > 1 {
        return parts
            .into_iter()
            .enumerate()
            .map(|(i, text)| TextBlock::new(file_id, i, text.to_string()))
            .collect();
    }

    // Fall back to sentence splitting
    let sentences: Vec<&str> = content
        .unicode_sentences()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if sentences.len() > 1 {
        return sentences
            .into_iter()
            .enumerate()
            .map(|(i, text)| TextBlock::new(file_id, i, text.to_string()))
            .collect();
    }

    // Single block fallback
    vec![TextBlock::new(file_id, 0, content.to_string())]
}
