//! LSP position plumbing: LSP ranges use UTF-16 code units, the model uses
//! byte offsets within lines.

use lsp_types::{Position, Range};

pub fn utf16_col(line: &str, byte: usize) -> u32 {
    let byte = byte.min(line.len());
    line[..byte].encode_utf16().count() as u32
}

pub fn byte_from_utf16(line: &str, col: u32) -> usize {
    let mut units = 0u32;
    for (byte, c) in line.char_indices() {
        if units >= col {
            return byte;
        }
        units += c.len_utf16() as u32;
    }
    line.len()
}

pub fn range(row: usize, line: &str, start_byte: usize, end_byte: usize) -> Range {
    Range {
        start: Position::new(row as u32, utf16_col(line, start_byte)),
        end: Position::new(row as u32, utf16_col(line, end_byte)),
    }
}

pub fn line_range(row: usize, line: &str) -> Range {
    range(row, line, 0, line.trim_end_matches('\r').len())
}

/// A range covering the entire document, for full-text replacement edits.
pub fn full_range(lines: &[String]) -> Range {
    let last = lines.len().saturating_sub(1);
    Range {
        start: Position::new(0, 0),
        end: Position::new(last as u32, utf16_col(&lines[last], lines[last].len())),
    }
}
