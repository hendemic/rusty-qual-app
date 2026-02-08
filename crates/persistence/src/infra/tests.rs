use super::*;
use app_core::domain::*;
use app_core::ports::ProjectRepository;
use chrono::Utc;
use tempfile::tempdir;

// ===== Test Helpers =====

/// Creates a minimal QualProject for testing
fn create_test_project() -> QualProject {
    let now = Utc::now();
    QualProject::new("Test Project".to_string(), 1, now, now)
}

/// Creates a CodeBook and FileList populated with code defs, themes, and qual codes.
/// Returns both because creating qual codes requires valid BlockIds from FileList.
fn create_populated_data() -> (CodeBook, FileList) {
    let mut codebook = CodeBook::new();
    let mut filelist = FileList::new();

    // Create a theme
    let theme_id = codebook.create_theme("Interview Themes".to_string(), 1);

    // Create code defs, one with theme and one without
    let code_def_with_theme = codebook.create_code_def(
        "Positive Sentiment".to_string(),
        2,
        Some(theme_id),
    );
    let _code_def_no_theme = codebook.create_code_def(
        "Negative Sentiment".to_string(),
        3,
        None,
    );

    // Add a file with a block
    let file_id = FileId::generate();
    let block = TextBlock::new(file_id, 0, "I really enjoyed it very much".to_string());
    let block_id = block.id;
    filelist.add_file(file_id, "interview_01.txt".to_string(), "/path/to/interview_01.txt".to_string(), FileType::PlainText, vec![block]);

    // Apply a qual code
    let highlight = Highlight::new(block_id, 2, 19);
    codebook.apply_code(
        code_def_with_theme,
        highlight,
        "really enjoyed it".to_string(),
        "I ".to_string(),
        " very much".to_string(),
    );

    (codebook, filelist)
}

// ===== Tests for save_project and load_project =====

mod save_load {
    use super::*;

    #[tokio::test]
    async fn test_save_project_creates_valid_json_file() {
        // Setup
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("project.json");
        let repo = JsonRepository::new();
        let project = create_test_project();
        let codebook = CodeBook::new();
        let filelist = FileList::new();

        // Execute
        repo.save_project(&file_path, project, codebook, filelist)
            .await
            .expect("save_project should succeed");

        // Assert: file exists and contains valid JSON
        let contents = tokio::fs::read_to_string(&file_path)
            .await
            .expect("should be able to read saved file");
        let parsed: serde_json::Value =
            serde_json::from_str(&contents).expect("file should contain valid JSON");
        assert!(parsed.is_object(), "top-level JSON should be an object");
        assert!(parsed.get("project").is_some(), "JSON should have a 'project' key");
        assert!(parsed.get("codebook").is_some(), "JSON should have a 'codebook' key");
        assert!(parsed.get("filemanager").is_some(), "JSON should have a 'filemanager' key");
    }

    #[tokio::test]
    async fn test_save_load_roundtrip_preserves_data() {
        // Setup
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("roundtrip.json");
        let repo = JsonRepository::new();
        let project = create_test_project();
        let codebook = CodeBook::new();
        let filelist = FileList::new();

        let original_name = project.name().to_string();
        let original_version = project.schema_version();

        // Execute: save then load
        repo.save_project(&file_path, project, codebook, filelist)
            .await
            .expect("save should succeed");

        let (loaded_project, _loaded_codebook, _loaded_filelist) = repo
            .load_project(&file_path)
            .await
            .expect("load should succeed");

        // Assert: project metadata preserved
        assert_eq!(loaded_project.name(), original_name, "project name should be preserved");
        assert_eq!(
            loaded_project.schema_version(),
            original_version,
            "schema version should be preserved"
        );
    }

    #[tokio::test]
    async fn test_save_load_roundtrip_with_codebook_data() {
        // Setup
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("codebook_roundtrip.json");
        let repo = JsonRepository::new();
        let project = create_test_project();
        let (codebook, filelist) = create_populated_data();

        // Capture counts before save
        let code_def_count = codebook.get_all_code_defs().count();
        let theme_count = codebook.get_all_themes().count();
        let qual_code_count = codebook.get_all_qual_codes().len();

        // Execute: save then load
        repo.save_project(&file_path, project, codebook, filelist)
            .await
            .expect("save should succeed");

        let (_loaded_project, loaded_codebook, _loaded_filelist) = repo
            .load_project(&file_path)
            .await
            .expect("load should succeed");

        // Assert: codebook data preserved
        assert_eq!(
            loaded_codebook.get_all_code_defs().count(),
            code_def_count,
            "code def count should be preserved"
        );
        assert_eq!(
            loaded_codebook.get_all_themes().count(),
            theme_count,
            "theme count should be preserved"
        );
        assert_eq!(
            loaded_codebook.get_all_qual_codes().len(),
            qual_code_count,
            "qual code count should be preserved"
        );

        // Verify theme name survived roundtrip
        let themes: Vec<&ThemeDef> = loaded_codebook.get_all_themes().collect();
        assert_eq!(themes[0].name(), "Interview Themes");

        // Verify code def names survived roundtrip
        let code_defs: Vec<&CodeDef> = loaded_codebook.get_all_code_defs().collect();
        let names: Vec<&str> = code_defs.iter().map(|cd| cd.name()).collect();
        assert!(names.contains(&"Positive Sentiment"), "should contain Positive Sentiment code def");
        assert!(names.contains(&"Negative Sentiment"), "should contain Negative Sentiment code def");

        // Verify qual code snippet survived roundtrip
        let qual_codes = loaded_codebook.get_all_qual_codes();
        assert_eq!(qual_codes[0].snippet(), "really enjoyed it");
    }

    #[tokio::test]
    async fn test_load_project_nonexistent_file_returns_error() {
        // Setup
        let dir = tempdir().unwrap();
        let nonexistent_path = dir.path().join("does_not_exist.json");
        let repo = JsonRepository::new();

        // Execute
        let result = repo.load_project(&nonexistent_path).await;

        // Assert
        assert!(result.is_err(), "loading nonexistent file should return error");
    }

    #[tokio::test]
    async fn test_load_project_invalid_json_returns_error() {
        // Setup: write garbage to a file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid.json");
        tokio::fs::write(&file_path, "this is not valid json {{{")
            .await
            .expect("should write garbage file");
        let repo = JsonRepository::new();

        // Execute
        let result = repo.load_project(&file_path).await;

        // Assert
        assert!(result.is_err(), "loading invalid JSON should return error");
    }

    #[tokio::test]
    async fn test_save_project_atomic_write_no_tmp_left() {
        // Setup
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("atomic.json");
        let tmp_path = dir.path().join("atomic.json.tmp");
        let repo = JsonRepository::new();
        let project = create_test_project();
        let codebook = CodeBook::new();
        let filelist = FileList::new();

        // Execute
        repo.save_project(&file_path, project, codebook, filelist)
            .await
            .expect("save should succeed");

        // Assert: the final file exists, temp file does not
        assert!(file_path.exists(), "project file should exist after save");
        assert!(!tmp_path.exists(), "temp file should not remain after save");
    }
}

// ===== Tests for LocalFileHandler =====

mod local_file_handler {
    use super::*;
    use app_core::ports::FileHandler;

    mod detect_type {
        use super::*;

        #[tokio::test]
        async fn test_detect_type_txt_returns_plain_text() {
            // Setup
            let handler = LocalFileHandler::new();
            let path = Path::new("document.txt");

            // Execute
            let result = handler.detect_type(path).await;

            // Assert
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), FileType::PlainText));
        }

        #[tokio::test]
        async fn test_detect_type_md_returns_markdown() {
            // Setup
            let handler = LocalFileHandler::new();
            let path = Path::new("notes.md");

            // Execute
            let result = handler.detect_type(path).await;

            // Assert
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), FileType::Markdown));
        }

        #[tokio::test]
        async fn test_detect_type_markdown_extension_returns_markdown() {
            // Setup
            let handler = LocalFileHandler::new();
            let path = Path::new("readme.markdown");

            // Execute
            let result = handler.detect_type(path).await;

            // Assert
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), FileType::Markdown));
        }

        #[tokio::test]
        async fn test_detect_type_pdf_returns_pdf() {
            // Setup
            let handler = LocalFileHandler::new();
            let path = Path::new("report.pdf");

            // Execute
            let result = handler.detect_type(path).await;

            // Assert
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), FileType::Pdf));
        }

        #[tokio::test]
        async fn test_detect_type_unknown_extension_returns_other() {
            // Setup
            let handler = LocalFileHandler::new();
            let path = Path::new("data.xyz");

            // Execute
            let result = handler.detect_type(path).await;

            // Assert
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), FileType::Other));
        }

        #[tokio::test]
        async fn test_detect_type_no_extension_returns_other() {
            // Setup
            let handler = LocalFileHandler::new();
            let path = Path::new("Makefile");

            // Execute
            let result = handler.detect_type(path).await;

            // Assert
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), FileType::Other));
        }
    }

    mod read_file_content {
        use super::*;

        #[tokio::test]
        async fn test_read_file_content_plain_text_returns_content() {
            // Setup
            let dir = tempdir().unwrap();
            let file_path = dir.path().join("sample.txt");
            let expected = "Hello, world!\nSecond line.";
            std::fs::write(&file_path, expected).unwrap();
            let handler = LocalFileHandler::new();

            // Execute
            let result = handler.read_file_content(&file_path).await;

            // Assert
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }

        #[tokio::test]
        async fn test_read_file_content_nonexistent_returns_error() {
            // Setup
            let dir = tempdir().unwrap();
            let missing = dir.path().join("does_not_exist.txt");
            let handler = LocalFileHandler::new();

            // Execute
            let result = handler.read_file_content(&missing).await;

            // Assert
            assert!(result.is_err(), "Reading nonexistent file should return error");
        }

        #[tokio::test]
        async fn test_read_file_content_pdf_returns_error() {
            // Setup: create a file with .pdf extension (content doesn't matter, stub rejects all PDFs)
            let dir = tempdir().unwrap();
            let pdf_path = dir.path().join("document.pdf");
            std::fs::write(&pdf_path, "fake pdf content").unwrap();
            let handler = LocalFileHandler::new();

            // Execute
            let result = handler.read_file_content(&pdf_path).await;

            // Assert
            assert!(result.is_err(), "Reading PDF should return error (not yet implemented)");
            let err_msg = format!("{}", result.unwrap_err());
            assert!(
                err_msg.contains("PDF support not yet implemented"),
                "Error should mention PDF not implemented, got: {}",
                err_msg
            );
        }
    }
}
