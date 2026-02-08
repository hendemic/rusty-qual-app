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

async fn create_controller_with_handler(handler: MockFileHandler) -> AppController<MockProjectRepo, MockFileHandler, MockConfigStore> {
    let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
    AppController::new(state, MockProjectRepo, handler, MockConfigStore)
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

// ===== File Action Tests =====

mod file_actions {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    /// Creates a stale FileId by adding and immediately removing a file.
    fn create_stale_file_id() -> FileId {
        let mut fl = FileList::new();
        let id = fl.add_file("stale.txt".to_string(), FileType::PlainText);
        fl.remove_file(id).unwrap();
        id
    }

    /// Creates a real temp file so that canonicalize() succeeds in the handlers.
    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("should create temp file");
        f.write_all(content.as_bytes()).expect("should write to temp file");
        f.flush().expect("should flush temp file");
        f
    }

    #[tokio::test]
    async fn test_add_file_returns_file_added() {
        // Setup
        let controller = create_test_controller().await;
        let tmp = create_temp_file("hello");

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::AddFile(tmp.path().to_path_buf())))
            .await;

        // Assert
        assert!(result.is_ok(), "AddFile should succeed");
        match result.unwrap() {
            ActionResult::FileAdded(_id) => {} // expected
            _ => panic!("Expected FileAdded"),
        }
    }

    #[tokio::test]
    async fn test_add_file_duplicate_path_returns_error() {
        // Setup
        let controller = create_test_controller().await;
        let tmp = create_temp_file("hello");
        let path = tmp.path().to_path_buf();

        // Add once
        let first = controller
            .handle_action(Action::File(FileAction::AddFile(path.clone())))
            .await;
        assert!(first.is_ok(), "First AddFile should succeed");

        // Execute: add same path again
        let result = controller
            .handle_action(Action::File(FileAction::AddFile(path)))
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
            Ok(_) => panic!("Expected error for duplicate AddFile"),
        }
    }

    #[tokio::test]
    async fn test_add_file_marks_state_modified() {
        // Setup
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));

        // Load a project first so mark_modified transitions Loaded -> Modified
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
            .handle_action(Action::File(FileAction::AddFile(tmp.path().to_path_buf())))
            .await
            .unwrap();

        // Assert: project state should be Modified
        let s = state.read().unwrap();
        assert!(
            matches!(s.project, DataState::Modified(_)),
            "Project state should be Modified after AddFile"
        );
    }

    #[tokio::test]
    async fn test_load_file_returns_file_loaded() {
        // Setup: controller with multi-paragraph content so blocks are created
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

        // Add a file first
        let tmp = create_temp_file("ignored by mock");
        let add_result = controller
            .handle_action(Action::File(FileAction::AddFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match add_result {
            ActionResult::FileAdded(id) => id,
            _ => panic!("Expected FileAdded"),
        };

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::LoadFile(file_id)))
            .await;

        // Assert
        assert!(result.is_ok(), "LoadFile should succeed");
        match result.unwrap() {
            ActionResult::FileLoaded(id) => assert_eq!(id, file_id),
            _ => panic!("Expected FileLoaded"),
        }

        // Verify blocks were created with correct file_id
        let s = state.read().unwrap();
        let file = s.filemanager.file(file_id).unwrap();
        let blocks = file.blocks().expect("File should have loaded blocks");
        assert_eq!(blocks.len(), 3, "Should have 3 paragraph blocks");
        for block in blocks {
            assert_eq!(block.file_id, file_id, "Block file_id should match");
        }
    }

    #[tokio::test]
    async fn test_load_file_nonexistent_id_returns_error() {
        // Setup
        let controller = create_test_controller().await;
        let fake_id = create_stale_file_id();

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::LoadFile(fake_id)))
            .await;

        // Assert
        assert!(result.is_err(), "LoadFile with nonexistent ID should error");
    }

    #[tokio::test]
    async fn test_remove_file_returns_file_removed() {
        // Setup
        let controller = create_test_controller().await;
        let tmp = create_temp_file("content");
        let add_result = controller
            .handle_action(Action::File(FileAction::AddFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match add_result {
            ActionResult::FileAdded(id) => id,
            _ => panic!("Expected FileAdded"),
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
        // Setup: controller with content, add file, load it, apply a qual code, then remove
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

        let tmp = create_temp_file("ignored");
        let add_result = controller
            .handle_action(Action::File(FileAction::AddFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match add_result {
            ActionResult::FileAdded(id) => id,
            _ => panic!("Expected FileAdded"),
        };

        // Load the file to get blocks
        controller
            .handle_action(Action::File(FileAction::LoadFile(file_id)))
            .await
            .unwrap();

        // Apply a qual code referencing a block from this file
        {
            let mut s = state.write().unwrap();
            let blocks = s.filemanager.file(file_id).unwrap().blocks().unwrap();
            let block_id = blocks[0].id;
            let code_def_id = s.codebook.create_code_def("Test Code".to_string(), 1, None);
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

        // Add and then remove a file
        let tmp = create_temp_file("content");
        let add_result = controller
            .handle_action(Action::File(FileAction::AddFile(tmp.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match add_result {
            ActionResult::FileAdded(id) => id,
            _ => panic!("Expected FileAdded"),
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
    async fn test_reattach_file_updates_path_and_clears_state() {
        // Setup
        let handler = MockFileHandler::new()
            .with_content("Some content.\n\nAnother para.");
        let state = Arc::new(RwLock::new(AppState::new(DataState::Empty, AppConfig::default())));
        let controller = AppController::new(
            state.clone(),
            MockProjectRepo,
            handler,
            MockConfigStore,
        )
        .await
        .unwrap();

        // Add and load a file
        let tmp1 = create_temp_file("original");
        let add_result = controller
            .handle_action(Action::File(FileAction::AddFile(tmp1.path().to_path_buf())))
            .await
            .unwrap();
        let file_id = match add_result {
            ActionResult::FileAdded(id) => id,
            _ => panic!("Expected FileAdded"),
        };

        controller
            .handle_action(Action::File(FileAction::LoadFile(file_id)))
            .await
            .unwrap();

        // Verify file has loaded blocks
        {
            let s = state.read().unwrap();
            assert!(s.filemanager.file(file_id).unwrap().blocks().is_some(), "File should be loaded");
        }

        // Execute: reattach to a new path
        let tmp2 = create_temp_file("new content");
        let new_path = tmp2.path().to_path_buf();
        let canonical_new = new_path.canonicalize().unwrap();

        let result = controller
            .handle_action(Action::File(FileAction::ReattachFile(file_id, new_path)))
            .await;

        // Assert
        assert!(result.is_ok(), "ReattachFile should succeed");
        match result.unwrap() {
            ActionResult::FileReattached(id) => assert_eq!(id, file_id),
            _ => panic!("Expected FileReattached"),
        }

        let s = state.read().unwrap();
        let file = s.filemanager.file(file_id).unwrap();
        assert_eq!(
            file.path(),
            canonical_new.to_string_lossy().as_ref(),
            "Path should be updated to new canonical path"
        );
        assert!(
            file.blocks().is_none(),
            "Data state should be cleared (Empty) after reattach"
        );
    }

    #[tokio::test]
    async fn test_reattach_file_nonexistent_id_returns_error() {
        // Setup
        let controller = create_test_controller().await;
        let tmp = create_temp_file("content");
        let fake_id = create_stale_file_id();

        // Execute
        let result = controller
            .handle_action(Action::File(FileAction::ReattachFile(fake_id, tmp.path().to_path_buf())))
            .await;

        // Assert
        assert!(result.is_err(), "ReattachFile with nonexistent ID should error");
    }
}

// ===== File Processing Tests =====

/// Creates a FileId for testing by adding a dummy file to a FileList.
fn test_file_id() -> FileId {
    let mut fl = FileList::new();
    fl.add_file("test.txt".to_string(), FileType::PlainText)
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
