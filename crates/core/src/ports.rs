use crate::domain::*;
use crate::application::*;
use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ProjectRepository {
    async fn new_project(
        &self,
        path: &Path,
        name: String,
        codebook: &CodeBook,
        files: &FileList
    ) -> Result<QualProject, ProjectError>;
    async fn save_project(&self, path: &Path, project: QualProject, codebook: CodeBook, files: FileList) -> Result<(), ProjectError>;
    async fn load_project(&self, path: &Path) -> Result<(QualProject, CodeBook, FileList), ProjectError>;
}

#[async_trait]
pub trait FileLoader {
    async fn load_file(&self, path: &Path) -> Result<(String, FileType)>;
    async fn load_file_metadata(&self, path: &Path) -> Result<FileType>;
}

#[async_trait]
pub trait ConfigStore {
    async fn load_config(&self) -> Result<AppConfig>;
    async fn save_config(&self) -> Result<()>;
    async fn config_exists(&self) -> bool;
}
