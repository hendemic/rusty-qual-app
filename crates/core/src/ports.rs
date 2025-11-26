use crate::domain::*;
use crate::application::*;
use std::path::Path;
use anyhow::Result;

pub trait ProjectRepository {
    fn new_project(
        &self,
        path: &Path,
        name: String,
        codebook: &CodeBook,
        files: &FileList
    ) -> Result<QualProject>;
    fn save_project(&self, path: &Path, project: QualProject, codebook: &CodeBook, files: &FileList) -> Result<()>;
    fn load_project(&self, path: &Path) -> Result<(QualProject, CodeBook, FileList)>;
}

pub trait FileLoader {
    fn load_file(&self, path: &Path) -> Result<(String, FileType)>;
    fn load_file_metadata(&self, path: &Path) -> Result<FileType>;
}

pub trait ConfigStore {
    fn load_config(&self) -> Result<AppConfig>;
    fn save_config(&self) -> Result<()>;
    fn config_exists(&self) -> bool;
}
