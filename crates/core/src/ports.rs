use crate::domain::*;
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
pub trait FileHandler{

    //need to figure out how this relates to the insert_file function in project repo. might just be load_file for now.
    async fn add_file(&self,  file_list: FileList, path: &Path) -> Result<(QualFile, FileType)>;
    async fn load_file(&self, file: FileId) -> Result<Vec<TextBlock>>;
}

#[async_trait]
pub trait ConfigStore {
    async fn load_config(&self) -> Result<AppConfig>;
    async fn save_config(&self) -> Result<()>;
    async fn config_exists(&self) -> bool;
}
