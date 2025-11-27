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
    ) -> Result<QualProject, ProjectError>;
    async fn save_project(&self, path: &Path, project: QualProject, codebook: CodeBook, files: FileList) -> Result<(), ProjectError>;
    async fn load_project(&self, path: &Path) -> Result<(QualProject, CodeBook, FileList), ProjectError>;
}

#[async_trait]
pub trait FileLoader {
    async fn add_file(&self,  file_list: FileList, path: &Path) -> Result<(QualFile, FileType)>;
    async fn load_file(&self, file: FileId) -> Result<Vec<TextBlock>, FileError>;
}

#[async_trait]
pub trait ConfigStore {
    async fn load_config(&self) -> Result<AppConfig>;
    async fn save_config(&self) -> Result<()>;
    async fn config_exists(&self) -> bool;
}
