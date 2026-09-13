use std::fmt;
use std::path::{Path, PathBuf};

/// Stable identity of a source file inside a [`SourceMap`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileId(pub u32);

/// A half-open byte range in an Aerofyl source file.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    pub file: FileId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(file: FileId, start: usize, end: usize) -> Self {
        Self { file, start, end }
    }

    pub const fn empty(file: FileId, offset: usize) -> Self {
        Self::new(file, offset, offset)
    }

    pub fn join(self, other: Self) -> Self {
        debug_assert_eq!(self.file, other.file);
        Self::new(
            self.file,
            self.start.min(other.start),
            self.end.max(other.end),
        )
    }
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    id: FileId,
    path: PathBuf,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    fn new(id: FileId, path: PathBuf, text: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        );
        Self {
            id,
            path,
            text,
            line_starts,
        }
    }

    pub const fn id(&self) -> FileId {
        self.id
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Resolves a byte offset to a one-based line and Unicode-scalar column.
    pub fn line_column(&self, offset: usize) -> Option<SourceLocation> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return None;
        }
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let line_start = self.line_starts[line_index];
        let column = self.text[line_start..offset].chars().count() + 1;
        Some(SourceLocation {
            line: line_index + 1,
            column,
        })
    }

    pub fn line_text(&self, one_based_line: usize) -> Option<&str> {
        let start = *self.line_starts.get(one_based_line.checked_sub(1)?)?;
        let end = self
            .line_starts
            .get(one_based_line)
            .copied()
            .unwrap_or(self.text.len());
        Some(self.text[start..end].trim_end_matches(['\n', '\r']))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: impl Into<PathBuf>, text: impl Into<String>) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files
            .push(SourceFile::new(id, path.into(), text.into()));
        id
    }

    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "file#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_unicode_columns() {
        let mut sources = SourceMap::new();
        let id = sources.add("test.fyl", "one\nαx\n");
        let file = sources.get(id).unwrap();
        assert_eq!(
            file.line_column(6),
            Some(SourceLocation { line: 2, column: 2 })
        );
        assert_eq!(file.line_text(2), Some("αx"));
    }
}
