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
    ) -> Result<QualProject>;
    async fn save_project(
            &self,
            path: &Path,
            project: QualProject,
            codebook: CodeBook,
            filemanager: FileList
        ) -> Result<()>;
    async fn load_project(&self, path: &Path) -> Result<(QualProject, CodeBook, FileList)>;

    //leaving these commented until I have the app + infra implementing them
    //async fn insert_code_def(&self, code: CodeDef) -> Result<()>;
    //async fn insert_theme_def(&self, code: ThemeDef) -> Result<()>;
    //async fn insert_qual_code(&self, code: QualCode);
    //async fn insert_file(&self, code: QualFile) -> Result<()>;
    //async fn delete_code_def(&self, id: CodeDefId) -> Result<()>;
    //async fn delete_theme_def(&self, id: ThemeId) -> Result<()>;
    //async fn delete_qual_code(&self, id: QualCodeId) -> Result<()>;
    //async fn delete_file(&self, id: FileId) -> Result<()>;
}

#[async_trait]
pub trait FileLoader {

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
