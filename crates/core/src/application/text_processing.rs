use crate::domain::{FileId, FileList, Highlight, TextBlock};
use unicode_segmentation::UnicodeSegmentation;
use anyhow::{Result, Context, bail};

/// Splits file content into `TextBlock`s using a lossless cascade strategy:
/// 1. Split on paragraph boundaries (two or more consecutive newlines)
/// 2. If only one result, fall back to line boundaries (single newlines)
/// 3. If still only one result, fall back to sentence boundaries
/// 4. Single block fallback
///
/// Invariant: `blocks.iter().map(|b| format!("{}{}", b.content, b.separator)).collect::<String>() == original`
pub fn split_into_blocks(file_id: FileId, content: &str) -> Vec<TextBlock> {
    if content.is_empty() {
        return Vec::new();
    }

    // Try paragraph splitting first
    let blocks = split_on_paragraph_boundaries(file_id, content);
    if blocks.len() > 1 {
        return blocks;
    }

    // Fall back to line splitting
    let blocks = split_on_line_boundaries(file_id, content);
    if blocks.len() > 1 {
        return blocks;
    }

    // Fall back to sentence splitting
    let blocks = split_on_sentence_boundaries(file_id, content);
    if blocks.len() > 1 {
        return blocks;
    }

    // Single block fallback
    vec![TextBlock::new(file_id, 0, content.to_string(), String::new())]
}

/// Split on paragraph boundaries: sequences of two or more newlines.
/// Content is text between boundaries, separator is the newline sequence.
fn split_on_paragraph_boundaries(file_id: FileId, content: &str) -> Vec<TextBlock> {
    let bytes = content.as_bytes();
    let len = bytes.len();

    // Find all paragraph boundary spans (runs of 2+ newlines)
    let mut boundaries: Vec<(usize, usize)> = Vec::new(); // (start, end) of each \n\n+ run
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\n' {
            let start = i;
            while i < len && bytes[i] == b'\n' {
                i += 1;
            }
            let run_len = i - start;
            if run_len >= 2 {
                boundaries.push((start, i));
            }
        } else {
            i += 1;
        }
    }

    if boundaries.is_empty() {
        return vec![TextBlock::new(file_id, 0, content.to_string(), String::new())];
    }

    let mut blocks = Vec::new();
    let mut pos = 0;

    for (seq, &(boundary_start, boundary_end)) in boundaries.iter().enumerate() {
        let block_content = &content[pos..boundary_start];
        let separator = &content[boundary_start..boundary_end];
        blocks.push(TextBlock::new(
            file_id,
            seq,
            block_content.to_string(),
            separator.to_string(),
        ));
        pos = boundary_end;
    }

    // Last block: remaining content with empty separator
    let last_content = &content[pos..];
    blocks.push(TextBlock::new(
        file_id,
        blocks.len(),
        last_content.to_string(),
        String::new(),
    ));

    blocks
}

/// Split on single newline boundaries.
fn split_on_line_boundaries(file_id: FileId, content: &str) -> Vec<TextBlock> {
    let bytes = content.as_bytes();

    // Find all single-newline positions
    let mut newline_positions: Vec<usize> = Vec::new();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte == b'\n' {
            newline_positions.push(i);
        }
    }

    if newline_positions.is_empty() {
        return vec![TextBlock::new(file_id, 0, content.to_string(), String::new())];
    }

    let mut blocks = Vec::new();
    let mut pos = 0;

    for (seq, &nl_pos) in newline_positions.iter().enumerate() {
        let block_content = &content[pos..nl_pos];
        let separator = "\n";
        blocks.push(TextBlock::new(
            file_id,
            seq,
            block_content.to_string(),
            separator.to_string(),
        ));
        pos = nl_pos + 1;
    }

    // Last block: remaining content after final newline
    let last_content = &content[pos..];
    blocks.push(TextBlock::new(
        file_id,
        blocks.len(),
        last_content.to_string(),
        String::new(),
    ));

    blocks
}

/// Split on sentence boundaries using unicode segmentation.
/// Uses pointer arithmetic on `unicode_sentences()` slices for exact byte positions,
/// ensuring lossless reconstruction.
fn split_on_sentence_boundaries(file_id: FileId, content: &str) -> Vec<TextBlock> {
    // unicode_sentences() returns sentence slices that include trailing whitespace.
    // We use pointer arithmetic to find exact offsets within the original string,
    // then derive separators from any gaps between consecutive sentences.
    let sentences: Vec<&str> = content.unicode_sentences().collect();
    if sentences.len() <= 1 {
        return vec![TextBlock::new(file_id, 0, content.to_string(), String::new())];
    }

    let mut blocks = Vec::new();
    let mut pos: usize = 0;

    for (seq, sentence) in sentences.iter().enumerate() {
        // SAFETY: unicode_sentences() returns subslices of the original `content` str,
        // so each sentence pointer is guaranteed to lie within [content.as_ptr(), content.as_ptr() + content.len()].
        // Subtracting the base pointer yields a valid byte offset on a char boundary.
        let sentence_ptr = sentence.as_ptr() as usize;
        let content_ptr = content.as_ptr() as usize;
        let sentence_start = sentence_ptr - content_ptr;
        let sentence_end = sentence_start + sentence.len();

        let is_last = seq == sentences.len() - 1;

        // Block content includes any skipped text (leading whitespace) before this sentence
        let block_content = if is_last {
            &content[pos..]
        } else {
            &content[pos..sentence_end]
        };

        let separator = if is_last {
            ""
        } else {
            // Next sentence pointer tells us where separator ends
            let next = sentences[seq + 1];
            let next_ptr = next.as_ptr() as usize;
            let next_start = next_ptr - content_ptr;
            &content[sentence_end..next_start]
        };

        blocks.push(TextBlock::new(
            file_id,
            seq,
            block_content.to_string(),
            separator.to_string(),
        ));

        if !is_last {
            let next = sentences[seq + 1];
            let next_ptr = next.as_ptr() as usize;
            pos = next_ptr - content_ptr;
        }
    }

    blocks
}

/// Extracts the highlighted text snippet from blocks referenced by a Highlight.
pub fn extract_snippet(filemanager: &FileList, highlight: &Highlight) -> Result<String> {
    if !highlight.is_multi_block() {
        // Single-block case
        let block = filemanager.find_block(highlight.start_block())
            .ok_or_else(|| anyhow::anyhow!("Start block not found"))?;
        if highlight.start() > block.content.len() || highlight.end() > block.content.len() {
            bail!("Highlight offsets out of bounds: start={}, end={}, block_len={}",
                highlight.start(), highlight.end(), block.content.len());
        }
        if !block.content.is_char_boundary(highlight.start()) || !block.content.is_char_boundary(highlight.end()) {
            bail!("Highlight offsets not on UTF-8 char boundary: start={}, end={}",
                highlight.start(), highlight.end());
        }
        Ok(block.content[highlight.start()..highlight.end()].to_string())
    } else {
        // Multi-block case
        let blocks = filemanager.find_blocks_in_range(highlight.start_block(), highlight.end_block())
            .context("Failed to find blocks in highlight range")?;

        let mut result = String::new();
        for (i, block) in blocks.iter().enumerate() {
            if i == 0 {
                // First block: from start offset to end of content
                if highlight.start() > block.content.len() {
                    bail!("Start offset out of bounds: start={}, block_len={}",
                        highlight.start(), block.content.len());
                }
                if !block.content.is_char_boundary(highlight.start()) {
                    bail!("Start offset not on UTF-8 char boundary: start={}",
                        highlight.start());
                }
                result.push_str(&block.content[highlight.start()..]);
                result.push_str(&block.separator);
            } else if i == blocks.len() - 1 {
                // Last block: from beginning to end offset
                if highlight.end() > block.content.len() {
                    bail!("End offset out of bounds: end={}, block_len={}",
                        highlight.end(), block.content.len());
                }
                if !block.content.is_char_boundary(highlight.end()) {
                    bail!("End offset not on UTF-8 char boundary: end={}",
                        highlight.end());
                }
                result.push_str(&block.content[..highlight.end()]);
            } else {
                // Intermediate blocks: full content + separator
                result.push_str(&block.content);
                result.push_str(&block.separator);
            }
        }
        Ok(result)
    }
}

/// Extracts context text before the highlight start position.
/// Walks backward from the start offset to the nearest sentence boundary or block start.
pub fn extract_context_before(filemanager: &FileList, highlight: &Highlight) -> String {
    let Some(block) = filemanager.find_block(highlight.start_block()) else {
        return String::new();
    };
    if highlight.start() > block.content.len() || !block.content.is_char_boundary(highlight.start()) {
        return String::new();
    }
    let before = &block.content[..highlight.start()];
    if before.is_empty() {
        return String::new();
    }

    // Walk backward to find nearest sentence boundary (". ", "? ", "! ")
    let sentence_endings = [". ", "? ", "! "];
    let mut best_pos = 0; // default to block start
    for ending in &sentence_endings {
        if let Some(pos) = before.rfind(ending) {
            let candidate = pos + ending.len();
            if candidate > best_pos {
                best_pos = candidate;
            }
        }
    }

    before[best_pos..].to_string()
}

/// Extracts context text after the highlight end position.
/// Walks forward from the end offset to the nearest sentence boundary or block end.
pub fn extract_context_after(filemanager: &FileList, highlight: &Highlight) -> String {
    let Some(block) = filemanager.find_block(highlight.end_block()) else {
        return String::new();
    };
    if highlight.end() > block.content.len() || !block.content.is_char_boundary(highlight.end()) {
        return String::new();
    }
    let after = &block.content[highlight.end()..];
    if after.is_empty() {
        return String::new();
    }

    // Walk forward to find nearest sentence boundary (". ", "? ", "! ")
    let sentence_endings = [". ", "? ", "! "];
    let mut best_pos = after.len(); // default to block end
    for ending in &sentence_endings {
        if let Some(pos) = after.find(ending) {
            let candidate = pos + ending.len();
            if candidate < best_pos {
                best_pos = candidate;
            }
        }
    }

    after[..best_pos].to_string()
}
