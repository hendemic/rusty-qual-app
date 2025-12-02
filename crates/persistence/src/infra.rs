#![allow(dead_code, unused_variables)]
use app_core::domain::{QualProject, CodeBook, FileList, ProjectError};
use app_core::ports::ProjectRepository;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool};
use std::sync::Arc;
use chrono::Utc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::fs;
use anyhow::Result;


pub struct JsonRepository {
    file_path: PathBuf,
    autosave_active: Arc<AtomicBool>,
    autosave_pending: Arc<AtomicBool>,
    manual_save_active: Arc<AtomicBool>,
    manual_save_pending: Arc<AtomicBool>,
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    project: QualProject,
    codebook: CodeBook,
    filemanager: FileList,
}

#[async_trait]
impl ProjectRepository for JsonRepository {
    async fn new_project(&self, path: &Path, name: String) -> Result<QualProject> {
        let now = Utc::now();
        let project = QualProject::new(
            name,
            1,
            now,
            now
        );

        let codebook = CodeBook::new();
        let filemanager = FileList::new();

        let project_file = ProjectFile {
            project: project.clone(),
            codebook,
            filemanager,
        };

        let json = serde_json::to_string_pretty(&project_file)
            .map_err(|e| ProjectError::Save(format!("Serialization failed: {}", e)))?;

        fs::write(path, json)
            .await
            .map_err(|e| ProjectError::Save(format!("Failed to write file: {}", e)))?;

        Ok(project)
    }
    async fn save_project(&self, path: &Path, project: QualProject, codebook: CodeBook, filemanager: FileList) -> Result<()> {
        todo!("implment save");
    }
    async fn load_project(&self, path: &Path) -> Result<(QualProject, CodeBook, FileList)> {
        todo!("implmement load");
    }
}
