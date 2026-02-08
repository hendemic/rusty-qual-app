use crate::domain::{QualProject, CodeBook, FileList, FileType, AppConfig};
use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ProjectRepository {
    async fn new_project(
        &self,
        path: &Path,
        name: String,
    ) -> Result<QualProject>;
    async fn save_project(
        &self,
        path: &Path,
        project: QualProject,
        codebook: CodeBook,
        filemanager: FileList
    ) -> Result<()>;
    async fn load_project(&self, path: &Path) -> Result<(QualProject, CodeBook, FileList)>;
    async fn autosave(
        &self,
        path: &Path,
        project: QualProject,
        codebook: CodeBook,
        filemanager: FileList
    ) -> Result<()>;
}

#[async_trait]
pub trait FileHandler {
    async fn read_file_content(&self, path: &Path) -> Result<String>;
    async fn detect_type(&self, path: &Path) -> Result<FileType>;
}

#[async_trait]
pub trait ConfigStore {
    async fn load_config(&self) -> Result<AppConfig>;
    async fn save_config(&self) -> Result<()>;
    async fn config_exists(&self) -> bool;
}
