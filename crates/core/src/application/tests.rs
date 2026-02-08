use super::*;
use super::split_into_blocks;

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

struct MockFileHandler {
    content: String,
    file_type: FileType,
}

impl MockFileHandler {
    fn new() -> Self {
        Self {
            content: "Default test content.".to_string(),
            file_type: FileType::PlainText,
        }
    }

    fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }
}

#[async_trait]
impl FileHandler for MockFileHandler {
    async fn read_file_content(&self, _path: &Path) -> Result<String> {
        Ok(self.content.clone())
    }

    async fn detect_type(&self, _path: &Path) -> Result<FileType> {
        Ok(self.file_type)
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
    AppController::new(state, MockProjectRepo, MockFileHandler::new(), MockConfigStore)
        .await
        .expect("controller creation should succeed")
}

/// Creates a test controller with a project already loaded (so is_loaded() returns true).
async fn create_test_controller_with_project() -> AppController<MockProjectRepo, MockFileHandler, MockConfigStore> {
    let controller = create_test_controller().await;
    controller.handle_action(Action::Project(ProjectAction::NewProject {
        path: PathBuf::from("/tmp/test_project.json"),
        name: "Test Project".to_string(),
    })).await.expect("NewProject should succeed");
    controller
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

// ===== File Action Tests =====

mod file_actions {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    /// Creates a stale FileId that doesn't exist in any FileList.
    fn create_stale_file_id() -> FileId {
        FileId::generate()
    }

    /// Creates a real temp file so that canonicalize() succeeds in the handlers.
    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("should create temp file");
        f.write_all(content.as_bytes()).expect("should write to temp file");
        f.flush().expect("should flush temp file");
        f
    }

    #[tokio::test]
    async fn test_import_file_returns_file_imported() {
        // Setup
        let controller = create_test_controller_with_project().await;
        let tmp = create_temp_file("hello");

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::ImportFile(tmp.path().to_path_buf())))
            .await;

        // Assert
        assert!(result.is_ok(), "ImportFile should succeed");
        match result.unwrap() {
            ActionResult::FileImported(_id) => {} // expected
            _ => panic!("Expected FileImported"),
        }
    }

    #[tokio::test]
    async fn test_import_file_creates_blocks() {
        // Setup: controller with multi-paragraph content
        let handler = MockFileHandler::new()
            .with_content("Para one.\n\nPara two.\n\nPara three.");
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(),
            MockProjectRepo,
            handler,
            MockConfigStore,
        )
        .await
        .unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/test_blocks.json"),
            name: "Test".to_string(),
        })).await.unwrap();

        let tmp = create_temp_file("ignored by mock");

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::ImportFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match result {
            ActionResult::FileImported(id) => id,
            _ => panic!("Expected FileImported"),
        };

        // Assert: blocks were created with correct file_id
        let s = state.read().unwrap();
        let file = s.filemanager.file(file_id).unwrap();
        let blocks = file.blocks().expect("File should have blocks after import");
        assert_eq!(blocks.len(), 3, "Should have 3 paragraph blocks");
        for block in blocks {
            assert_eq!(block.file_id, file_id, "Block file_id should match");
        }
    }

    #[tokio::test]
    async fn test_import_file_sets_name_from_filename() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(),
            MockProjectRepo,
            MockFileHandler::new(),
            MockConfigStore,
        )
        .await
        .unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/test_name.json"),
            name: "Test".to_string(),
        })).await.unwrap();

        let tmp = create_temp_file("content");

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::ImportFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match result {
            ActionResult::FileImported(id) => id,
            _ => panic!("Expected FileImported"),
        };

        // Assert: name should be the filename portion of the path
        let s = state.read().unwrap();
        let file = s.filemanager.file(file_id).unwrap();
        assert!(!file.name().is_empty(), "File name should not be empty");
    }

    #[tokio::test]
    async fn test_import_file_duplicate_path_returns_error() {
        // Setup
        let controller = create_test_controller_with_project().await;
        let tmp = create_temp_file("hello");
        let path = tmp.path().to_path_buf();

        // Import once
        let first = controller
            .handle_action(Action::File(FileAction::ImportFile(path.clone())))
            .await;
        assert!(first.is_ok(), "First ImportFile should succeed");

        // Execute: import same path again
        let result = controller
            .handle_action(Action::File(FileAction::ImportFile(path)))
            .await;

        // Assert
        match result {
            Err(err) => {
                let err_msg = format!("{}", err);
                assert!(
                    err_msg.contains("already added"),
                    "Error should mention duplicate, got: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error for duplicate ImportFile"),
        }
    }

    #[tokio::test]
    async fn test_import_file_marks_state_modified() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(),
            MockProjectRepo,
            MockFileHandler::new(),
            MockConfigStore,
        )
        .await
        .expect("controller creation should succeed");

        let new_action = Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/test_mark_mod.json"),
            name: "Test".to_string(),
        });
        controller.handle_action(new_action).await.unwrap();

        let tmp = create_temp_file("content");

        // Execute
        controller
            .handle_action(Action::File(FileAction::ImportFile(tmp.path().to_path_buf())))
            .await
            .unwrap();

        // Assert: project state should be Modified
        let s = state.read().unwrap();
        assert!(
            matches!(s.project, DataState::Modified(_)),
            "Project state should be Modified after ImportFile"
        );
    }

    #[tokio::test]
    async fn test_remove_file_returns_file_removed() {
        // Setup
        let controller = create_test_controller_with_project().await;
        let tmp = create_temp_file("content");
        let import_result = controller
            .handle_action(Action::File(FileAction::ImportFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match import_result {
            ActionResult::FileImported(id) => id,
            _ => panic!("Expected FileImported"),
        };

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::RemoveFile(file_id)))
            .await;

        // Assert
        assert!(result.is_ok(), "RemoveFile should succeed");
        match result.unwrap() {
            ActionResult::FileRemoved(id) => assert_eq!(id, file_id),
            _ => panic!("Expected FileRemoved"),
        }
    }

    #[tokio::test]
    async fn test_remove_file_cascades_qual_codes() {
        // Setup: controller with content, import file, apply a qual code, then remove
        let handler = MockFileHandler::new()
            .with_content("Block one.\n\nBlock two.");
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(),
            MockProjectRepo,
            handler,
            MockConfigStore,
        )
        .await
        .unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/test_cascade.json"),
            name: "Test".to_string(),
        })).await.unwrap();

        let tmp = create_temp_file("ignored");
        let import_result = controller
            .handle_action(Action::File(FileAction::ImportFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match import_result {
            ActionResult::FileImported(id) => id,
            _ => panic!("Expected FileImported"),
        };

        // Apply a qual code referencing a block from this file
        {
            let mut s = state.write().unwrap();
            let blocks = s.filemanager.file(file_id).unwrap().blocks().unwrap();
            let block_id = blocks[0].id;
            let code_def_id = s.codebook.create_code_def("Test Code".to_string(), 1, None).unwrap();
            let highlight = Highlight::new(block_id, 0, 5);
            s.codebook.apply_code(code_def_id, highlight, "Block".to_string(), "".to_string(), " one.".to_string());
        }

        // Verify qual code exists before removal
        {
            let s = state.read().unwrap();
            assert_eq!(s.codebook.get_all_qual_codes().len(), 1, "Should have 1 qual code before remove");
        }

        // Execute: remove the file
        controller
            .handle_action(Action::File(FileAction::RemoveFile(file_id)))
            .await
            .unwrap();

        // Assert: qual codes for that file should be removed
        let s = state.read().unwrap();
        assert_eq!(
            s.codebook.get_all_qual_codes().len(),
            0,
            "Qual codes should be cascade-deleted when file is removed"
        );
    }

    #[tokio::test]
    async fn test_remove_file_marks_state_modified() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(),
            MockProjectRepo,
            MockFileHandler::new(),
            MockConfigStore,
        )
        .await
        .unwrap();

        // Create a project so state is Loaded
        controller
            .handle_action(Action::Project(ProjectAction::NewProject {
                path: PathBuf::from("/tmp/test_rm_mod.json"),
                name: "Test".to_string(),
            }))
            .await
            .unwrap();

        // Save to move back to Loaded state
        controller
            .handle_action(Action::Project(ProjectAction::SaveProject))
            .await
            .unwrap();

        // Import a file
        let tmp = create_temp_file("content");
        let import_result = controller
            .handle_action(Action::File(FileAction::ImportFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match import_result {
            ActionResult::FileImported(id) => id,
            _ => panic!("Expected FileImported"),
        };

        // Save again to reset to Loaded
        controller
            .handle_action(Action::Project(ProjectAction::SaveProject))
            .await
            .unwrap();

        // Execute
        controller
            .handle_action(Action::File(FileAction::RemoveFile(file_id)))
            .await
            .unwrap();

        // Assert
        let s = state.read().unwrap();
        assert!(
            matches!(s.project, DataState::Modified(_)),
            "Project state should be Modified after RemoveFile"
        );
    }

    #[tokio::test]
    async fn test_reload_file_updates_path_and_blocks() {
        // Setup
        let handler = MockFileHandler::new()
            .with_content("New content.\n\nNew paragraph.");
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(),
            MockProjectRepo,
            handler,
            MockConfigStore,
        )
        .await
        .unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/test_reload.json"),
            name: "Test".to_string(),
        })).await.unwrap();

        // Import a file
        let tmp1 = create_temp_file("original");
        let import_result = controller
            .handle_action(Action::File(FileAction::ImportFile(tmp1.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match import_result {
            ActionResult::FileImported(id) => id,
            _ => panic!("Expected FileImported"),
        };

        // Execute: reload from a new path
        let tmp2 = create_temp_file("new content");
        let new_path = tmp2.path().to_path_buf();
        let canonical_new = new_path.canonicalize().unwrap();

        let result = controller
            .handle_action(Action::File(FileAction::ReloadFile(file_id, new_path)))
            .await;

        // Assert
        assert!(result.is_ok(), "ReloadFile should succeed");
        match result.unwrap() {
            ActionResult::FileReloaded(id) => assert_eq!(id, file_id),
            _ => panic!("Expected FileReloaded"),
        }

        let s = state.read().unwrap();
        let file = s.filemanager.file(file_id).unwrap();
        assert_eq!(
            file.path(),
            canonical_new.to_string_lossy().as_ref(),
            "Path should be updated to new canonical path"
        );
        // Blocks should be present (re-ingested from new source)
        let blocks = file.blocks().expect("File should have blocks after reload");
        assert_eq!(blocks.len(), 2, "Should have 2 paragraph blocks from new content");
    }

    #[tokio::test]
    async fn test_reload_file_nonexistent_id_returns_error() {
        // Setup
        let controller = create_test_controller().await;
        let tmp = create_temp_file("content");
        let fake_id = create_stale_file_id();

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::ReloadFile(fake_id, tmp.path().to_path_buf())))
            .await;

        // Assert
        assert!(result.is_err(), "ReloadFile with nonexistent ID should error");
    }
}

// ===== File Processing Tests =====

/// Creates a FileId for testing.
fn test_file_id() -> FileId {
    FileId::generate()
}

mod paragraph_splitting {
    use super::*;

    #[test]
    fn test_split_paragraphs_double_newline_produces_multiple_blocks() {
        // Setup
        let file_id = test_file_id();
        let content = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";

        // Execute
        let blocks = split_into_blocks(file_id, content);

        // Assert
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].content, "First paragraph.");
        assert_eq!(blocks[1].content, "Second paragraph.");
        assert_eq!(blocks[2].content, "Third paragraph.");
    }
}

mod line_splitting {
    use super::*;

    #[test]
    fn test_split_fallback_to_lines_single_paragraph_multiple_lines() {
        // Setup
        let file_id = test_file_id();
        let content = "Line one.\nLine two.\nLine three.";

        // Execute
        let blocks = split_into_blocks(file_id, content);

        // Assert
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].content, "Line one.");
        assert_eq!(blocks[1].content, "Line two.");
        assert_eq!(blocks[2].content, "Line three.");
    }
}

mod sentence_splitting {
    use super::*;

    #[test]
    fn test_split_fallback_to_sentences_single_line_multiple_sentences() {
        // Setup
        let file_id = test_file_id();
        let content = "First sentence. Second sentence. Third sentence.";

        // Execute
        let blocks = split_into_blocks(file_id, content);

        // Assert
        assert!(blocks.len() >= 2, "Should split into multiple sentence blocks, got {}", blocks.len());
    }
}

mod single_block {
    use super::*;

    #[test]
    fn test_split_single_block_no_boundaries_returns_one_block() {
        // Setup
        let file_id = test_file_id();
        let content = "Just a single word";

        // Execute
        let blocks = split_into_blocks(file_id, content);

        // Assert
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Just a single word");
    }
}

mod empty_content {
    use super::*;

    #[test]
    fn test_split_empty_content_returns_empty_vec() {
        // Setup
        let file_id = test_file_id();

        // Execute
        let blocks = split_into_blocks(file_id, "");

        // Assert
        assert!(blocks.is_empty());
    }
}

mod sequence_numbers {
    use super::*;

    #[test]
    fn test_split_correct_sequence_numbers_zero_indexed() {
        // Setup
        let file_id = test_file_id();
        let content = "Para one.\n\nPara two.\n\nPara three.";

        // Execute
        let blocks = split_into_blocks(file_id, content);

        // Assert
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].sequence, 0);
        assert_eq!(blocks[1].sequence, 1);
        assert_eq!(blocks[2].sequence, 2);
    }
}

mod file_id_correctness {
    use super::*;

    #[test]
    fn test_split_correct_file_id_on_all_blocks() {
        // Setup
        let file_id = test_file_id();
        let content = "Block A.\n\nBlock B.\n\nBlock C.";

        // Execute
        let blocks = split_into_blocks(file_id, content);

        // Assert
        for block in &blocks {
            assert_eq!(block.file_id, file_id, "All blocks should have the same file_id");
        }
    }
}

// ===== Tests for AppState::is_loaded =====

mod is_loaded_tests {
    use super::*;

    #[test]
    fn test_is_loaded_returns_false_for_empty() {
        // Setup
        let state = AppState::new(DataState::Empty, AppConfig::default());

        // Assert
        assert!(!state.is_loaded(), "Empty state should not be loaded");
    }

    #[test]
    fn test_is_loaded_returns_true_for_loaded() {
        // Setup
        let now = chrono::Utc::now();
        let project = QualProject::new("Test".to_string(), 1, now, now);
        let ctx = ProjectContext::new(PathBuf::from("/tmp/test.json"), project);
        let state = AppState::new(DataState::Loaded(ctx), AppConfig::default());

        // Assert
        assert!(state.is_loaded(), "Loaded state should be loaded");
    }

    #[test]
    fn test_is_loaded_returns_true_for_modified() {
        // Setup
        let now = chrono::Utc::now();
        let project = QualProject::new("Test".to_string(), 1, now, now);
        let ctx = ProjectContext::new(PathBuf::from("/tmp/test.json"), project);
        let state = AppState::new(DataState::Modified(ctx), AppConfig::default());

        // Assert
        assert!(state.is_loaded(), "Modified state should be loaded");
    }

    #[test]
    fn test_is_loaded_returns_false_for_error() {
        // Setup
        let state = AppState::new(DataState::Error, AppConfig::default());

        // Assert
        assert!(!state.is_loaded(), "Error state should not be loaded");
    }
}

// ===== Tests for schema actions =====

mod schema_action_tests {
    use super::*;

    /// Helper: creates a controller and loads a project so is_loaded() passes
    async fn setup_controller_with_project() -> AppController<MockProjectRepo, MockFileHandler, MockConfigStore> {
        let controller = create_test_controller().await;
        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/schema_test.json"),
            name: "Schema Test".to_string(),
        })).await.expect("NewProject should succeed");
        controller
    }

    // ----- Happy path tests -----

    #[tokio::test]
    async fn test_create_code_happy_path_returns_code_created() {
        // Setup
        let controller = setup_controller_with_project().await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "TestCode".to_string(),
            color: 1,
            theme_id: None,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::CodeCreated(_) => {}
            _ => panic!("Expected CodeCreated"),
        }
    }

    #[tokio::test]
    async fn test_rename_code_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let code_id = match controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Original".to_string(), color: 1, theme_id: None,
        })).await.unwrap() {
            ActionResult::CodeCreated(id) => id,
            _ => panic!("Expected CodeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::RenameCode {
            id: code_id,
            name: "Renamed".to_string(),
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_update_code_color_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let code_id = match controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Code".to_string(), color: 1, theme_id: None,
        })).await.unwrap() {
            ActionResult::CodeCreated(id) => id,
            _ => panic!("Expected CodeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::UpdateCodeColor {
            id: code_id,
            color: 42,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_delete_code_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let code_id = match controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "ToDelete".to_string(), color: 1, theme_id: None,
        })).await.unwrap() {
            ActionResult::CodeCreated(id) => id,
            _ => panic!("Expected CodeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::DeleteCode {
            id: code_id,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_create_theme_happy_path_returns_theme_created() {
        // Setup
        let controller = setup_controller_with_project().await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "TestTheme".to_string(),
            color: 5,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::ThemeCreated(_) => {}
            _ => panic!("Expected ThemeCreated"),
        }
    }

    #[tokio::test]
    async fn test_rename_theme_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let theme_id = match controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "Original".to_string(), color: 1,
        })).await.unwrap() {
            ActionResult::ThemeCreated(id) => id,
            _ => panic!("Expected ThemeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::RenameTheme {
            id: theme_id,
            name: "Renamed".to_string(),
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_update_theme_color_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let theme_id = match controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "Theme".to_string(), color: 1,
        })).await.unwrap() {
            ActionResult::ThemeCreated(id) => id,
            _ => panic!("Expected ThemeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::UpdateThemeColor {
            id: theme_id,
            color: 99,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_delete_theme_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let theme_id = match controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "ToDelete".to_string(), color: 1,
        })).await.unwrap() {
            ActionResult::ThemeCreated(id) => id,
            _ => panic!("Expected ThemeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::DeleteTheme {
            id: theme_id,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    // ----- No project loaded tests -----

    #[tokio::test]
    async fn test_schema_action_returns_error_when_no_project_loaded() {
        // Setup: controller with no project loaded
        let controller = create_test_controller().await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Code".to_string(), color: 1, theme_id: None,
        })).await;

        // Assert
        match result {
            Err(err) => {
                let err_msg = format!("{}", err);
                assert!(
                    err_msg.contains("No project loaded"),
                    "Error should mention no project loaded, got: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error when no project loaded"),
        }
    }

    // ----- Error cases for nonexistent IDs -----

    /// Creates a CodeDefId that no longer exists in the codebook (create then delete).
    async fn stale_code_def_id(controller: &AppController<MockProjectRepo, MockFileHandler, MockConfigStore>) -> CodeDefId {
        let id = match controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Stale".to_string(), color: 0, theme_id: None,
        })).await.unwrap() {
            ActionResult::CodeCreated(id) => id,
            _ => panic!("Expected CodeCreated"),
        };
        controller.handle_action(Action::Schema(SchemaAction::DeleteCode { id }))
            .await.unwrap();
        id
    }

    /// Creates a ThemeId that no longer exists in the codebook (create then delete).
    async fn stale_theme_id(controller: &AppController<MockProjectRepo, MockFileHandler, MockConfigStore>) -> ThemeId {
        let id = match controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "Stale".to_string(), color: 0,
        })).await.unwrap() {
            ActionResult::ThemeCreated(id) => id,
            _ => panic!("Expected ThemeCreated"),
        };
        controller.handle_action(Action::Schema(SchemaAction::DeleteTheme { id }))
            .await.unwrap();
        id
    }

    #[tokio::test]
    async fn test_rename_code_nonexistent_id_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_id = stale_code_def_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::RenameCode {
            id: fake_id, name: "X".to_string(),
        })).await;

        // Assert
        assert!(result.is_err(), "RenameCode with nonexistent ID should fail");
    }

    #[tokio::test]
    async fn test_update_code_color_nonexistent_id_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_id = stale_code_def_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::UpdateCodeColor {
            id: fake_id, color: 1,
        })).await;

        // Assert
        assert!(result.is_err(), "UpdateCodeColor with nonexistent ID should fail");
    }

    #[tokio::test]
    async fn test_delete_code_nonexistent_id_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_id = stale_code_def_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::DeleteCode {
            id: fake_id,
        })).await;

        // Assert
        assert!(result.is_err(), "DeleteCode with nonexistent ID should fail");
    }

    #[tokio::test]
    async fn test_rename_theme_nonexistent_id_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_id = stale_theme_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::RenameTheme {
            id: fake_id, name: "X".to_string(),
        })).await;

        // Assert
        assert!(result.is_err(), "RenameTheme with nonexistent ID should fail");
    }

    #[tokio::test]
    async fn test_update_theme_color_nonexistent_id_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_id = stale_theme_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::UpdateThemeColor {
            id: fake_id, color: 1,
        })).await;

        // Assert
        assert!(result.is_err(), "UpdateThemeColor with nonexistent ID should fail");
    }

    #[tokio::test]
    async fn test_delete_theme_nonexistent_id_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_id = stale_theme_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::DeleteTheme {
            id: fake_id,
        })).await;

        // Assert
        assert!(result.is_err(), "DeleteTheme with nonexistent ID should fail");
    }

    // ----- CreateCode with invalid theme_id -----

    #[tokio::test]
    async fn test_create_code_with_invalid_theme_id_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_theme = stale_theme_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Code".to_string(), color: 1, theme_id: Some(fake_theme),
        })).await;

        // Assert
        assert!(result.is_err(), "CreateCode with invalid theme_id should fail");
    }

    // ----- DeleteCode cascades QualCode removal -----

    #[tokio::test]
    async fn test_delete_code_cascades_qual_code_removal() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(), MockProjectRepo, MockFileHandler::new(), MockConfigStore,
        ).await.unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/cascade_test.json"),
            name: "Cascade".to_string(),
        })).await.unwrap();

        // Create a code def and apply qual codes
        let code_id = {
            let mut s = state.write().unwrap();
            let code_id = s.codebook.create_code_def("CascadeCode".to_string(), 1, None).unwrap();
            let file_id = FileId::generate();
            let block = TextBlock::new(file_id, 0, "test content".to_string());
            let block_id = block.id;
            s.filemanager.add_file(file_id, "test.txt".to_string(), "test.txt".to_string(), FileType::PlainText, vec![block]);
            let highlight = Highlight::new(block_id, 0, 5);
            s.codebook.apply_code(code_id, highlight, "test".to_string(), "".to_string(), " content".to_string());
            code_id
        };

        // Verify qual code exists
        {
            let s = state.read().unwrap();
            assert_eq!(s.codebook.get_all_qual_codes().len(), 1, "Should have 1 qual code before delete");
        }

        // Execute: delete the code def via schema action
        let result = controller.handle_action(Action::Schema(SchemaAction::DeleteCode {
            id: code_id,
        })).await;

        // Assert
        assert!(result.is_ok());
        let s = state.read().unwrap();
        assert_eq!(s.codebook.get_all_qual_codes().len(), 0, "Qual codes should be cascade-deleted");
        assert!(s.codebook.code_def(code_id).is_none(), "Code def should be removed");
    }

    // ----- AssignCodeToTheme / RemoveCodeFromTheme -----

    #[tokio::test]
    async fn test_assign_code_to_theme_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let theme_id = match controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "Theme".to_string(), color: 1,
        })).await.unwrap() {
            ActionResult::ThemeCreated(id) => id,
            _ => panic!("Expected ThemeCreated"),
        };
        let code_id = match controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Code".to_string(), color: 1, theme_id: None,
        })).await.unwrap() {
            ActionResult::CodeCreated(id) => id,
            _ => panic!("Expected CodeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::AssignCodeToTheme {
            code_id, theme_id,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_remove_code_from_theme_happy_path_returns_success() {
        // Setup
        let controller = setup_controller_with_project().await;
        let theme_id = match controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "Theme".to_string(), color: 1,
        })).await.unwrap() {
            ActionResult::ThemeCreated(id) => id,
            _ => panic!("Expected ThemeCreated"),
        };
        let code_id = match controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Code".to_string(), color: 1, theme_id: Some(theme_id),
        })).await.unwrap() {
            ActionResult::CodeCreated(id) => id,
            _ => panic!("Expected CodeCreated"),
        };

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::RemoveCodeFromTheme {
            code_id,
        })).await;

        // Assert
        assert!(result.is_ok());
        match result.unwrap() {
            ActionResult::Success => {}
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_assign_code_to_theme_nonexistent_code_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let theme_id = match controller.handle_action(Action::Schema(SchemaAction::CreateTheme {
            name: "Theme".to_string(), color: 1,
        })).await.unwrap() {
            ActionResult::ThemeCreated(id) => id,
            _ => panic!("Expected ThemeCreated"),
        };
        let fake_code = stale_code_def_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::AssignCodeToTheme {
            code_id: fake_code, theme_id,
        })).await;

        // Assert
        assert!(result.is_err(), "AssignCodeToTheme with nonexistent code should fail");
    }

    #[tokio::test]
    async fn test_assign_code_to_theme_nonexistent_theme_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let code_id = match controller.handle_action(Action::Schema(SchemaAction::CreateCode {
            name: "Code".to_string(), color: 1, theme_id: None,
        })).await.unwrap() {
            ActionResult::CodeCreated(id) => id,
            _ => panic!("Expected CodeCreated"),
        };
        let fake_theme = stale_theme_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::AssignCodeToTheme {
            code_id, theme_id: fake_theme,
        })).await;

        // Assert
        assert!(result.is_err(), "AssignCodeToTheme with nonexistent theme should fail");
    }

    #[tokio::test]
    async fn test_remove_code_from_theme_nonexistent_code_returns_error() {
        // Setup
        let controller = setup_controller_with_project().await;
        let fake_code = stale_code_def_id(&controller).await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::RemoveCodeFromTheme {
            code_id: fake_code,
        })).await;

        // Assert
        assert!(result.is_err(), "RemoveCodeFromTheme with nonexistent code should fail");
    }

    #[tokio::test]
    async fn test_assign_code_to_theme_no_project_loaded_returns_error() {
        // Setup: get valid IDs from a project, then use a fresh controller with no project
        let setup_controller = setup_controller_with_project().await;
        let code_id = stale_code_def_id(&setup_controller).await;
        let theme_id = stale_theme_id(&setup_controller).await;

        let controller = create_test_controller().await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::AssignCodeToTheme {
            code_id, theme_id,
        })).await;

        // Assert
        match result {
            Err(err) => {
                let err_msg = format!("{}", err);
                assert!(
                    err_msg.contains("No project loaded"),
                    "Error should mention no project loaded, got: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error when no project loaded"),
        }
    }

    #[tokio::test]
    async fn test_remove_code_from_theme_no_project_loaded_returns_error() {
        // Setup: get a valid ID from a project, then use a fresh controller with no project
        let setup_controller = setup_controller_with_project().await;
        let code_id = stale_code_def_id(&setup_controller).await;

        let controller = create_test_controller().await;

        // Execute
        let result = controller.handle_action(Action::Schema(SchemaAction::RemoveCodeFromTheme {
            code_id,
        })).await;

        // Assert
        match result {
            Err(err) => {
                let err_msg = format!("{}", err);
                assert!(
                    err_msg.contains("No project loaded"),
                    "Error should mention no project loaded, got: {}",
                    err_msg
                );
            }
            Ok(_) => panic!("Expected error when no project loaded"),
        }
    }

    #[tokio::test]
    async fn test_assign_code_to_theme_updates_code_def_theme_id() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(), MockProjectRepo, MockFileHandler::new(), MockConfigStore,
        ).await.unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/assign_verify.json"),
            name: "AssignVerify".to_string(),
        })).await.unwrap();

        let (theme_id, code_id) = {
            let mut s = state.write().unwrap();
            let tid = s.codebook.create_theme("Theme".to_string(), 1);
            let cid = s.codebook.create_code_def("Code".to_string(), 1, None).unwrap();
            (tid, cid)
        };

        // Verify code starts with no theme
        {
            let s = state.read().unwrap();
            assert_eq!(s.codebook.code_def(code_id).unwrap().theme_id(), None);
        }

        // Execute
        controller.handle_action(Action::Schema(SchemaAction::AssignCodeToTheme {
            code_id, theme_id,
        })).await.unwrap();

        // Assert: verify state was actually updated
        let s = state.read().unwrap();
        assert_eq!(
            s.codebook.code_def(code_id).unwrap().theme_id(),
            Some(theme_id),
            "Code's theme_id should be set after AssignCodeToTheme"
        );
    }

    #[tokio::test]
    async fn test_remove_code_from_theme_sets_theme_id_to_none() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(), MockProjectRepo, MockFileHandler::new(), MockConfigStore,
        ).await.unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/remove_verify.json"),
            name: "RemoveVerify".to_string(),
        })).await.unwrap();

        let (theme_id, code_id) = {
            let mut s = state.write().unwrap();
            let tid = s.codebook.create_theme("Theme".to_string(), 1);
            let cid = s.codebook.create_code_def("Code".to_string(), 1, Some(tid)).unwrap();
            (tid, cid)
        };

        // Verify code starts with theme assigned
        {
            let s = state.read().unwrap();
            assert_eq!(s.codebook.code_def(code_id).unwrap().theme_id(), Some(theme_id));
        }

        // Execute
        controller.handle_action(Action::Schema(SchemaAction::RemoveCodeFromTheme {
            code_id,
        })).await.unwrap();

        // Assert: verify theme_id is now None
        let s = state.read().unwrap();
        assert_eq!(
            s.codebook.code_def(code_id).unwrap().theme_id(),
            None,
            "Code's theme_id should be None after RemoveCodeFromTheme"
        );
    }

    // ----- DeleteTheme unassigns CodeDefs -----

    #[tokio::test]
    async fn test_delete_theme_unassigns_code_defs() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(), MockProjectRepo, MockFileHandler::new(), MockConfigStore,
        ).await.unwrap();

        controller.handle_action(Action::Project(ProjectAction::NewProject {
            path: PathBuf::from("/tmp/unassign_test.json"),
            name: "Unassign".to_string(),
        })).await.unwrap();

        // Create theme and code defs assigned to it
        let (theme_id, code_id_1, code_id_2) = {
            let mut s = state.write().unwrap();
            let tid = s.codebook.create_theme("Theme".to_string(), 1);
            let c1 = s.codebook.create_code_def("Code1".to_string(), 1, Some(tid)).unwrap();
            let c2 = s.codebook.create_code_def("Code2".to_string(), 2, Some(tid)).unwrap();
            (tid, c1, c2)
        };

        // Execute: delete the theme
        let result = controller.handle_action(Action::Schema(SchemaAction::DeleteTheme {
            id: theme_id,
        })).await;

        // Assert
        assert!(result.is_ok());
        let s = state.read().unwrap();
        assert!(s.codebook.theme(theme_id).is_none(), "Theme should be removed");
        // Code defs should still exist but with theme_id set to None
        let c1 = s.codebook.code_def(code_id_1).expect("Code1 should still exist");
        let c2 = s.codebook.code_def(code_id_2).expect("Code2 should still exist");
        assert_eq!(c1.theme_id(), None, "Code1 should be unassigned from theme");
        assert_eq!(c2.theme_id(), None, "Code2 should be unassigned from theme");
    }
}
