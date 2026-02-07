use app_core::domain::{QualProject, CodeBook, FileList, ProjectError};
use app_core::ports::ProjectRepository;

use std::path::{Path, PathBuf};
use chrono::Utc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::fs;
use anyhow::Result;


pub struct JsonRepository;

impl JsonRepository {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    project: QualProject,
    codebook: CodeBook,
    filemanager: FileList,
}

/// Writes contents to a temp file then atomically renames to the target path.
/// Prevents corruption if the process crashes mid-write.
async fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let mut tmp_os = path.as_os_str().to_os_string();
    tmp_os.push(".tmp");
    let tmp_path = PathBuf::from(tmp_os);

    fs::write(&tmp_path, contents)
        .await
        .map_err(|e| ProjectError::Save(format!("Failed to write temp file: {}", e)))?;

    fs::rename(&tmp_path, path)
        .await
        .map_err(|e| ProjectError::Save(format!("Failed to rename temp file: {}", e)))?;

    Ok(())
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

        atomic_write(path, &json).await?;

        Ok(project)
    }
    async fn save_project(&self, path: &Path, project: QualProject, codebook: CodeBook, filemanager: FileList) -> Result<()> {
        let project_file = ProjectFile {
            project,
            codebook,
            filemanager,
        };

        let json = serde_json::to_string_pretty(&project_file)
            .map_err(|e| ProjectError::Save(format!("Serialization failed: {}", e)))?;

        atomic_write(path, &json).await?;

        Ok(())
    }
    async fn load_project(&self, path: &Path) -> Result<(QualProject, CodeBook, FileList)> {
        let contents = fs::read_to_string(path)
            .await
            .map_err(|e| ProjectError::Load(format!("Failed to read file: {}", e)))?;

        let project_file: ProjectFile = serde_json::from_str(&contents)
            .map_err(|e| ProjectError::InvalidFormat(format!("Failed to parse project file: {}", e)))?;

        Ok((project_file.project, project_file.codebook, project_file.filemanager))
    }
    async fn autosave(&self, _path: &Path, _project: QualProject, _codebook: CodeBook, _filemanager: FileList) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
