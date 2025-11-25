#![allow(dead_code)]
use uuid::Uuid;
use indexmap::IndexMap;
use std::fmt;

// Unique Uuid's to enforce type safety and enable entities to reference each other.
// Relying on cross-referencing Uuid's to avoid lifetime shinanigans, and for look up performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeDefId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QualCodeId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemeId(Uuid);

#[derive(Debug)]
pub enum CodeBookError {
    CodeDefNotFound(CodeDefId),
    ThemeNotFound(ThemeId),
    QualCodeNotFound(QualCodeId),
    InvalidIndex { provided: usize, max: usize },
}

impl fmt::Display for CodeBookError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CodeBookError::CodeDefNotFound(id) => write!(f, "Code definition not found: {:?}", id),
            CodeBookError::ThemeNotFound(id) => write!(f, "Theme not found: {:?}", id),
            CodeBookError::QualCodeNotFound(id) => write!(f, "Qual code not found: {:?}", id),
            CodeBookError::InvalidIndex { provided, max } => {
                write!(f, "Invalid index: {} (max valid index is {})", provided, max)
            }
        }
    }
}

impl std::error::Error for CodeBookError {}

#[derive(Debug)]
pub enum FileListError {
    FileNotFound(FileId),
    InvalidIndex { provided: usize, max: usize },
}

impl fmt::Display for FileListError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileListError::FileNotFound(id) => write!(f, "File not found: {:?}", id),
            FileListError::InvalidIndex { provided, max } => {
                write!(f, "Invalid index: {} (max valid index is {})", provided, max)
            }
        }
    }
}


impl std::error::Error for FileListError {}

#[derive(Debug)]
pub enum FileError {
    Read(FileId),
    Write(FileId),
    Parse(FileId),
    Encoding(FileId),
    Unknown(FileId),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileError::Read(id) => write!(f, "Failed to read file: {:?}", id),
            FileError::Write(id) => write!(f, "Failed to write file: {:?}", id),
            FileError::Parse(id) => write!(f, "Failed to parse file: {:?}", id),
            FileError::Encoding(id) => write!(f, "Failed to encode file: {:?}", id),
            FileError::Unknown(id) => write!(f, "Unknown error for file: {:?}", id),
        }
    }
}

impl std::error::Error for FileError {}


pub struct Highlight {
    file_id: FileId,
    start: u64,
    end: u64,
}

impl Highlight {
    pub fn new(file_id: FileId, start: u64, end: u64) -> Self {
        let (start, end) = if start > end { (end, start) } else { (start, end) };
        Highlight { file_id, start, end }
    }
    pub fn len(&self) -> u64 {
        self.end - self.start
    }
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
    pub fn file_id(&self) -> FileId { self.file_id }
    pub fn start(&self) -> u64 { self.start }
    pub fn end(&self) -> u64 { self.end }
}

/// Definition of a code used in a project
pub struct CodeDef {
    pub id: CodeDefId,
    name: String,
    color: u8,
    theme_id: Option<ThemeId>,
}

impl CodeDef {
    // Private so that CodeBook owns construction and maintains ownership
    fn new(name: String, color: u8) -> Self {
        let id = CodeDefId(Uuid::new_v4());
        CodeDef { id, name, color, theme_id: None }
    }
    pub fn theme_id(&self) -> Option<ThemeId> { self.theme_id }
    pub fn set_theme_id(&mut self, theme_id: Option<ThemeId>) { self.theme_id = theme_id; }
    pub fn name(&self) -> &str { &self.name }
    pub fn color(&self) -> u8 { self.color }
}

/// Highlighted instance of a Code in a given file.
pub struct QualCode {
    pub id: QualCodeId,
    def_id: CodeDefId,
    highlight: Highlight,
    snippet: String,
}

impl QualCode {
    // Private so that CodeBook owns construction and maintains ownership
    fn new(id: QualCodeId, def_id: CodeDefId, highlight: Highlight, snippet: String) -> Self {
        QualCode { id, def_id, highlight, snippet }
    }
    pub fn def_id(&self) -> CodeDefId { self.def_id }
    pub fn file_id(&self) -> FileId { self.highlight.file_id() }
    pub fn position(&self) -> (u64, u64) { (self.highlight.start(), self.highlight.end()) }
    pub fn snippet(&self) -> &str { &self.snippet }
}

/// Collection of CodeDefs associated with a theme
pub struct ThemeDef {
    pub id: ThemeId,
    name: String,
    color: u8,
}

impl ThemeDef {
    // Private so that CodeBook owns construction and maintains ownership
    fn new(name: String, color: u8) -> Self {
        let id = ThemeId(Uuid::new_v4());
        ThemeDef { id, name, color }
    }
    pub fn name(&self) -> &str { &self.name }
    pub fn color(&self) -> u8 { self.color }
}

/// Codebook containing all code definitions themes, and vector of all QualCodes.
/// This struct is the core application state
/// This is used by application to create new codes, themes, and apply theme to text snippets in files.
pub struct CodeBook {
    code_defs: IndexMap<CodeDefId, CodeDef>,
    themes: IndexMap<ThemeId, ThemeDef>,
    qual_codes: Vec<QualCode>
}

// Core functions
impl CodeBook {
    pub fn new() -> Self {
        CodeBook {
            code_defs: IndexMap::new(),
            themes: IndexMap::new(),
            qual_codes: Vec::new(),
        }
    }

}

//CodeDef methods
impl CodeBook {
    pub fn create_code_def(&mut self, name: String, color: u8, theme_id: Option<ThemeId>) -> CodeDefId {
        let mut code_def = CodeDef::new(name, color);
        code_def.theme_id = theme_id;
        let id = code_def.id;
        self.code_defs.insert(id, code_def);
        id
    }
    pub fn code_def(&self, id: CodeDefId) -> Option<&CodeDef> { self.code_defs.get(&id) }

    pub fn remove_code_def(&mut self, id: CodeDefId) -> Result<CodeDef, CodeBookError> {
        self.qual_codes.retain(|qc| qc.def_id != id); //remove codes for the def first
        self.code_defs.shift_remove(&id)
            .ok_or(CodeBookError::CodeDefNotFound(id))
    }

    pub fn get_all_code_defs(&self) -> impl Iterator<Item = &CodeDef> { self.code_defs.values() }

    pub fn move_code_def_to_index(&mut self, id: CodeDefId, new_index: usize) -> Result<(), CodeBookError> {
        let current_index = self.code_defs.get_index_of(&id)
            .ok_or(CodeBookError::CodeDefNotFound(id))?;

        // Validate index is in range
        let max_index = self.code_defs.len().saturating_sub(1);
        if new_index > max_index {
            return Err(CodeBookError::InvalidIndex {
                provided: new_index,
                max: max_index
            });
        }

        self.code_defs.move_index(current_index, new_index);
        Ok(())
    }
    pub fn swap_code_defs(&mut self, index_a: usize, index_b: usize) -> Result<(), CodeBookError> {
        let max_index = self.code_defs.len().saturating_sub(1);
        if index_a > max_index  {
            return Err(CodeBookError::InvalidIndex { provided: index_a, max: max_index });
        }
        if index_b > max_index {
            return Err(CodeBookError::InvalidIndex { provided: index_b, max: max_index });
        }
        self.code_defs.swap_indices(index_a, index_b);
        Ok(())
    }
     pub fn sort_code_defs_by_name(&mut self) {
         self.code_defs.sort_by(|_, a, _, b| a.name().cmp(b.name()));
     }
}

//ThemeDef methods
impl CodeBook {
    pub fn create_theme(&mut self, name: String, color: u8) -> ThemeId {
        let theme = ThemeDef::new(name, color);
        let id = theme.id;
        self.themes.insert(id, theme);
        id
    }
    pub fn theme(&self, id: ThemeId) -> Option<&ThemeDef> { self.themes.get(&id) }

    pub fn remove_theme(&mut self, id: ThemeId) -> Result<ThemeDef, CodeBookError> {
        // Reset corresponding CodeDef references to None
        for code_def in self.code_defs.values_mut() {
            if code_def.theme_id == Some(id) {
                code_def.theme_id = None;
            }
        }
        self.themes.shift_remove(&id)
            .ok_or(CodeBookError::ThemeNotFound(id))
    }

    pub fn get_all_themes(&self) -> impl Iterator<Item = &ThemeDef> { self.themes.values() }

    pub fn get_codes_in_theme(&self, theme_id: ThemeId) -> impl Iterator<Item = &CodeDef> {
        self.code_defs.values().filter(move |cd| cd.theme_id == Some(theme_id))
    }
    pub fn get_top_level_codes(&self) -> impl Iterator<Item = &CodeDef> {
        self.code_defs.values().filter(|cd| cd.theme_id.is_none())
    }
    pub fn move_code_to_theme(&mut self, code_id: CodeDefId, theme_id: ThemeId) {
        if let Some(code_def) = self.code_defs.get_mut(&code_id) {
            code_def.theme_id = Some(theme_id);
        }
    }
    pub fn remove_code_from_theme(&mut self, code_id: CodeDefId) -> Result<(), CodeBookError> {
        let code_def = self.code_defs.get_mut(&code_id)
            .ok_or(CodeBookError::CodeDefNotFound(code_id))?;

        code_def.theme_id = None;  // Setting None to None is harmless
        Ok(())
    }
    pub fn move_theme_to_index(&mut self, id: ThemeId, new_index: usize) -> Result<(), CodeBookError> {
        let current_index = self.themes.get_index_of(&id)
            .ok_or(CodeBookError::ThemeNotFound(id))?;

        let max_index = self.themes.len().saturating_sub(1);
        if new_index > max_index {
            return Err(CodeBookError::InvalidIndex {
                provided: new_index,
                max: max_index
            });
        }
        self.themes.move_index(current_index, new_index);
        Ok(())
    }
    pub fn swap_themes(&mut self, index_a: usize, index_b: usize) -> Result<(), CodeBookError> {
        let max_index = self.themes.len().saturating_sub(1);
        if index_a > max_index  {
            return Err(CodeBookError::InvalidIndex { provided: index_a, max: max_index });
        }
        if index_b > max_index {
            return Err(CodeBookError::InvalidIndex { provided: index_b, max: max_index });
        }

        self.themes.swap_indices(index_a, index_b);
        Ok(())
    }
    pub fn sort_themes_by_name(&mut self) {
        self.themes.sort_by(|_, a, _, b| a.name().cmp(b.name()));
    }
}

//QualCode methods
impl CodeBook {
    pub fn apply_code(&mut self, code_def_id: CodeDefId, highlight: Highlight, snippet: String) -> QualCodeId {
        let id = QualCodeId(Uuid::new_v4());
        self.qual_codes.push(QualCode::new(id, code_def_id, highlight, snippet));
        id
    }
    pub fn remove_qual_code(&mut self, id: QualCodeId) -> Result<(), CodeBookError> {
        let pos = self.qual_codes.iter().position(|qc| qc.id == id)
            .ok_or(CodeBookError::QualCodeNotFound(id))?;

        self.qual_codes.remove(pos);
        Ok(())
    }
    pub fn get_codes_for_file(&self, file_id: FileId) -> impl Iterator<Item = &QualCode> {
        self.qual_codes.iter().filter(move |qc| qc.highlight.file_id() == file_id)
    }
    pub fn remove_codes_for_file(&mut self, file_id: FileId) {
        self.qual_codes.retain(|qc| qc.highlight.file_id() != file_id);
    }
    pub fn get_codes_for_def(&self, def_id: CodeDefId) -> impl Iterator<Item = &QualCode> {
        self.qual_codes.iter().filter(move |qc| qc.def_id == def_id)
    }

    pub fn get_all_qual_codes(&self) -> &[QualCode] {
        &self.qual_codes
    }
}


pub enum FileType {
    Pdf,
    PlainText,
    Markdown,
    RichText,
}

pub enum DataState<T> {
    Empty,
    Loaded(T),
    Modified(T),
    Error(T),
}

///File and its data and metadata
pub struct QualFile {
    pub id: FileId,
    path: String,
    data_state: DataState<String>,
    file_type: FileType,
}

impl QualFile {
    //Private new function
    fn new(path: String, file_type: FileType) -> Self {
        let id = FileId(Uuid::new_v4());
        QualFile { id, path, data_state: DataState::Empty, file_type }
    }

    pub fn path(&self) -> &str { &self.path }
    pub fn load_data(&mut self, data_state: DataState<String>) { self.data_state = data_state; }
    pub fn data(&self) -> Option<&str> {
        match &self.data_state {
            DataState::Loaded(content) | DataState::Modified(content) => Some(content),
            DataState::Empty | DataState::Error(_) => None,
        }
    }
    pub fn file_type(&self) -> &FileType { &self.file_type }
}

///Collection of files. Manages File addition, removal, retrieval, and ordering
pub struct FileList {
    files: IndexMap<FileId, QualFile>,
}

impl FileList {
    pub fn new() -> Self {
        FileList { files: IndexMap::new() }
    }
    pub fn add_file(&mut self, path: String, file_type: FileType) -> FileId {
        let file = QualFile::new(path, file_type);
        let id = file.id;
        self.files.insert(id, file);
        id
    }
    pub fn remove_file(&mut self, id: FileId) -> Result<(), FileListError> {
        self.files.shift_remove(&id)
            .ok_or(FileListError::FileNotFound(id))?;
        Ok(())
    }
    pub fn file(&self, id: FileId) -> Option<&QualFile> { self.files.get(&id) }
    pub fn get_all_files(&self) -> impl Iterator<Item = &QualFile> { self.files.values() }
    pub fn move_file_to_index(&mut self, id: FileId, new_index: usize) -> Result<(), FileListError> {
        let current_index = self.files.get_index_of(&id)
            .ok_or(FileListError::FileNotFound(id))?;

        let max_index = self.files.len().saturating_sub(1);
        if new_index > max_index {
            return Err(FileListError::InvalidIndex {
                provided: new_index,
                max: max_index,
            });
        }
        self.files.move_index(current_index, new_index);
        Ok(())
    }
    pub fn swap_files(&mut self, index_a: usize, index_b: usize) -> Result<(), FileListError> {
        let max_index = self.files.len().saturating_sub(1);
        if index_a > max_index {
            return Err(FileListError::InvalidIndex { provided: index_a, max: max_index });
        }
        if index_b > max_index {
            return Err(FileListError::InvalidIndex { provided: index_b, max: max_index });
        }
        self.files.swap_indices(index_a, index_b);
        Ok(())
    }
    pub fn sort_files_by_name(&mut self) {
        self.files.sort_by(|_, a, _, b| a.path().cmp(b.path()));
    }
    pub fn file_count(&self) -> usize { self.files.len() }
}
