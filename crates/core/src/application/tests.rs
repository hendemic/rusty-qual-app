use super::*;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use anyhow::Result;

// ===== Mock Implementations =====

struct MockProjectRepo;

#[async_trait]
impl ProjectRepository for MockProjectRepo {
    async fn new_project(&self, path: &Path, name: String) -> Result<QualProject> {
        let now = chrono::Utc::now();
        Ok(QualProject::new(name, 1, now, now))
    }

    async fn save_project(
        &self,
        _path: &Path,
        _project: QualProject,
        _codebook: CodeBook,
        _filemanager: FileList,
    ) -> Result<()> {
        Ok(())
    }

    async fn load_project(&self, _path: &Path) -> Result<(QualProject, CodeBook, FileList)> {
        let now = chrono::Utc::now();
        let project = QualProject::new("Loaded".to_string(), 1, now, now);
        Ok((project, CodeBook::new(), FileList::new()))
    }

    async fn autosave(&self, _path: &Path, _project: QualProject, _codebook: CodeBook, _filemanager: FileList) -> Result<()> {
        Ok(())
    }
}

struct MockFileHandler;

#[async_trait]
impl FileHandler for MockFileHandler {
    async fn add_file(&self, _file_list: FileList, _path: &Path) -> Result<(QualFile, FileType)> {
        unimplemented!("not under test")
    }

    async fn load_file(&self, _file: FileId) -> Result<Vec<TextBlock>> {
        unimplemented!("not under test")
    }
}

struct MockConfigStore;

#[async_trait]
impl ConfigStore for MockConfigStore {
    async fn load_config(&self) -> Result<AppConfig> {
        Ok(AppConfig::default())
    }

    async fn save_config(&self) -> Result<()> {
        Ok(())
    }

    async fn config_exists(&self) -> bool {
        true
    }
}

// ===== Test Helpers =====

async fn create_test_controller() -> AppController<MockProjectRepo, MockFileHandler, MockConfigStore> {
    let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
    AppController::new(state, MockProjectRepo, MockFileHandler, MockConfigStore)
        .await
        .expect("controller creation should succeed")
}

// ===== Save Guard Tests =====

mod save_guard {
    use super::*;

    #[tokio::test]
    async fn test_save_project_returns_success() {
        // Setup: create controller and first create a project so there is something to save
        let controller = create_test_controller().await;

        let new_action = Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/test_project.json"),
            name: "Test Project".to_string(),
        });
        let new_result = controller.handle_action(new_action).await;
        assert!(new_result.is_ok(), "NewProject should succeed");

        // Execute: save the project
        let save_action = Action::Project(ProjectAction::SaveProject);
        let result = controller.handle_action(save_action).await;

        // Assert
        assert!(result.is_ok(), "SaveProject should succeed");
        match result.unwrap() {
            ActionResult::Success => {} // expected
            _ => panic!("Expected ActionResult::Success"),
        }
    }

    #[tokio::test]
    async fn test_save_project_when_no_project_loaded_returns_error() {
        // Setup: controller with empty state (no project loaded)
        let controller = create_test_controller().await;

        // Execute: try to save with no project
        let save_action = Action::Project(ProjectAction::SaveProject);
        let result = controller.handle_action(save_action).await;

        // Assert: should return an error since no project is loaded
        match result {
            Err(err) => {
                let err_msg = format!("{}", err);
                assert!(
                    err_msg.contains("No project loaded"),
                    "Error should mention no project loaded, got: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error when saving with no project loaded"),
        }
    }
}
