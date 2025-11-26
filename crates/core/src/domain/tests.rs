// These tests were AI generated with human oversight and test design
// Philosophy on testing is that AI generated tests are better than no tests

use super::*;
use std::collections::HashMap;

// ===== Test Helpers =====

/// Creates an empty CodeBook for testing
fn create_test_codebook() -> CodeBook {
    CodeBook::new()
}

/// Creates a test file with a specified number of blocks
fn create_test_file(file_path: &str, num_blocks: usize) -> QualFile {
    let mut file = QualFile::new(file_path.to_string(), FileType::PlainText);
    let mut blocks = Vec::new();
    for i in 0..num_blocks {
        let block = TextBlock::new(file.id, i, format!("Block content {}", i));
        blocks.push(block);
    }
    file.set_data_state(DataState::Loaded(blocks));
    file
}

/// Applies a code to a specific block in the codebook
fn apply_test_code(
    codebook: &mut CodeBook,
    block_id: BlockId,
    code_def_id: CodeDefId,
    snippet: &str,
) -> QualCodeId {
    let highlight = Highlight::new(block_id, 0, 10);
    codebook.apply_code(code_def_id, highlight, snippet.to_string(), String::new(), String::new())
}

/// Builds a block-to-file mapping from a list of files
fn build_block_map(files: &[&QualFile]) -> HashMap<BlockId, FileId> {
    files
        .iter()
        .flat_map(|file| {
            file.blocks().unwrap_or(&[]).iter().map(move |block| (block.id, file.id))
        })
        .collect()
}


// ===== Tests for get_codes_for_file =====

mod code_retrieval {
    use super::*;

    #[test]
    fn test_get_codes_for_file_returns_all_codes_for_single_file() {
        // Setup: Create a file with 3 blocks and apply codes to all of them
        let mut codebook = create_test_codebook();
        let file = create_test_file("test.txt", 3);
        let block_map = build_block_map(&[&file]);

        let code_def_id = codebook.create_code_def("TestCode".to_string(), 1, None);

        // Apply 3 codes to different blocks in the same file
        let blocks = file.blocks().unwrap();
        let code_id_1 = apply_test_code(&mut codebook, blocks[0].id, code_def_id, "snippet1");
        let code_id_2 = apply_test_code(&mut codebook, blocks[1].id, code_def_id, "snippet2");
        let code_id_3 = apply_test_code(&mut codebook, blocks[2].id, code_def_id, "snippet3");

        // Execute: Get codes
        let codes: Vec<&QualCode> = codebook.get_codes_for_file(file.id, &block_map).collect();

        // Assert: Should return all 3 codes
        assert_eq!(codes.len(), 3, "Should return exactly 3 codes for the file");

        let code_ids: Vec<QualCodeId> = codes.iter().map(|c| c.id).collect();
        assert!(code_ids.contains(&code_id_1), "Should contain code_id_1");
        assert!(code_ids.contains(&code_id_2), "Should contain code_id_2");
        assert!(code_ids.contains(&code_id_3), "Should contain code_id_3");

        // Execute: Remove codes for this file
        codebook.remove_codes_for_file(file.id, &block_map);

        // Assert: No codes should remain for this file
        let codes_after_removal: Vec<&QualCode> = codebook.get_codes_for_file(file.id, &block_map).collect();
        assert_eq!(codes_after_removal.len(), 0, "Should have 0 codes after removal");
    }

    #[test]
    fn test_get_codes_for_file_isolates_between_files() {
        // Setup: Create two files with blocks and apply codes to each
        let mut codebook = create_test_codebook();
        let file_a = create_test_file("file_a.txt", 2);
        let file_b = create_test_file("file_b.txt", 2);
        let block_map = build_block_map(&[&file_a, &file_b]);

        let code_def_id = codebook.create_code_def("TestCode".to_string(), 1, None);

        // Apply codes to file A
        let blocks_a = file_a.blocks().unwrap();
        let code_a1 = apply_test_code(&mut codebook, blocks_a[0].id, code_def_id, "A1");
        let code_a2 = apply_test_code(&mut codebook, blocks_a[1].id, code_def_id, "A2");

        // Apply codes to file B
        let blocks_b = file_b.blocks().unwrap();
        let code_b1 = apply_test_code(&mut codebook, blocks_b[0].id, code_def_id, "B1");
        let code_b2 = apply_test_code(&mut codebook, blocks_b[1].id, code_def_id, "B2");

        // Execute: Get codes for file A
        let codes_a: Vec<&QualCode> = codebook.get_codes_for_file(file_a.id, &block_map).collect();
        let code_ids_a: Vec<QualCodeId> = codes_a.iter().map(|c| c.id).collect();

        // Execute: Get codes for file B
        let codes_b: Vec<&QualCode> = codebook.get_codes_for_file(file_b.id, &block_map).collect();
        let code_ids_b: Vec<QualCodeId> = codes_b.iter().map(|c| c.id).collect();

        // Assert: File A should only have its 2 codes
        assert_eq!(codes_a.len(), 2, "File A should have exactly 2 codes");
        assert!(code_ids_a.contains(&code_a1), "File A should contain code_a1");
        assert!(code_ids_a.contains(&code_a2), "File A should contain code_a2");
        assert!(!code_ids_a.contains(&code_b1), "File A should NOT contain code_b1");
        assert!(!code_ids_a.contains(&code_b2), "File A should NOT contain code_b2");

        // Assert: File B should only have its 2 codes
        assert_eq!(codes_b.len(), 2, "File B should have exactly 2 codes");
        assert!(code_ids_b.contains(&code_b1), "File B should contain code_b1");
        assert!(code_ids_b.contains(&code_b2), "File B should contain code_b2");
        assert!(!code_ids_b.contains(&code_a1), "File B should NOT contain code_a1");
        assert!(!code_ids_b.contains(&code_a2), "File B should NOT contain code_a2");

        // Execute: Remove codes for file A only
        codebook.remove_codes_for_file(file_a.id, &block_map);

        // Assert: File A should now have 0 codes
        let codes_a_after: Vec<&QualCode> = codebook.get_codes_for_file(file_a.id, &block_map).collect();
        assert_eq!(codes_a_after.len(), 0, "File A should have 0 codes after removal");

        // Assert: File B should still have its 2 codes (unaffected)
        let codes_b_after: Vec<&QualCode> = codebook.get_codes_for_file(file_b.id, &block_map).collect();
        let code_ids_b_after: Vec<QualCodeId> = codes_b_after.iter().map(|c| c.id).collect();
        assert_eq!(codes_b_after.len(), 2, "File B should still have 2 codes after removing A's codes");
        assert!(code_ids_b_after.contains(&code_b1), "File B should still contain code_b1");
        assert!(code_ids_b_after.contains(&code_b2), "File B should still contain code_b2");
    }

    #[test]
    fn test_get_codes_for_file_returns_empty_when_no_codes_applied() {
        // Setup: Create a file with blocks but don't apply any codes
        let codebook = create_test_codebook();
        let file = create_test_file("empty.txt", 3);
        let block_map = build_block_map(&[&file]);

        // Execute
        let codes: Vec<&QualCode> = codebook.get_codes_for_file(file.id, &block_map).collect();

        // Assert: Should return empty iterator
        assert_eq!(codes.len(), 0, "Should return 0 codes for file with no codes applied");
    }

    #[test]
    fn test_get_codes_for_file_with_empty_codebook() {
        // Setup: Empty codebook
        let codebook = create_test_codebook();
        let file = create_test_file("test.txt", 2);
        let block_map = build_block_map(&[&file]);

        // Execute
        let codes: Vec<&QualCode> = codebook.get_codes_for_file(file.id, &block_map).collect();

        // Assert: Should return empty
        assert_eq!(codes.len(), 0, "Empty codebook should return 0 codes");
    }

    #[test]
    fn test_get_codes_for_file_with_orphaned_block() {
        // Setup: Create a code with a block_id that doesn't exist in the map
        let mut codebook = create_test_codebook();
        let file = create_test_file("test.txt", 1);
        let code_def_id = codebook.create_code_def("TestCode".to_string(), 1, None);

        // Create a code with a valid block
        let blocks = file.blocks().unwrap();
        let _valid_code = apply_test_code(&mut codebook, blocks[0].id, code_def_id, "valid");

        // Create a code with an orphaned block (not in any file)
        let orphaned_block_id = BlockId(Uuid::new_v4());
        let _orphaned_code = apply_test_code(&mut codebook, orphaned_block_id, code_def_id, "orphaned");

        // Build map with only the valid file's blocks
        let block_map = build_block_map(&[&file]);

        // Execute: Get codes
        let codes: Vec<&QualCode> = codebook.get_codes_for_file(file.id, &block_map).collect();

        // Assert: Current behavior silently filters out orphaned blocks
        // Should only return the valid code, not the orphaned one
        assert_eq!(codes.len(), 1, "Should return only 1 valid code, orphaned code filtered out");

        // Execute: Remove codes for the file
        codebook.remove_codes_for_file(file.id, &block_map);

        // Assert: Valid code removed, but orphaned code should remain in codebook
        let codes_after: Vec<&QualCode> = codebook.get_codes_for_file(file.id, &block_map).collect();
        assert_eq!(codes_after.len(), 0, "Should have 0 codes for file after removal");

        // Verify the orphaned code still exists in the codebook (wasn't removed)
        assert_eq!(codebook.get_all_qual_codes().len(), 1, "Orphaned code should still exist in codebook");

        // NOTE: This test documents current behavior (silent filtering).
        // Future enhancement: Consider panicking on orphaned blocks to enforce data integrity.
    }

    // ===== Tests for remove_code_def (cascade delete) =====

    #[test]
    fn test_remove_code_def_cascades_to_qual_codes() {
        // Setup: Create a codebook with 2 code defs and apply codes from both
        let mut codebook = create_test_codebook();
        let file = create_test_file("test.txt", 4);

        let code_def_1 = codebook.create_code_def("Code1".to_string(), 1, None);
        let code_def_2 = codebook.create_code_def("Code2".to_string(), 2, None);

        // Apply 2 codes from code_def_1
        let blocks = file.blocks().unwrap();
        let code_1a = apply_test_code(&mut codebook, blocks[0].id, code_def_1, "1a");
        let code_1b = apply_test_code(&mut codebook, blocks[1].id, code_def_1, "1b");

        // Apply 2 codes from code_def_2
        let code_2a = apply_test_code(&mut codebook, blocks[2].id, code_def_2, "2a");
        let code_2b = apply_test_code(&mut codebook, blocks[3].id, code_def_2, "2b");

        // Verify we have 4 codes total
        assert_eq!(codebook.get_all_qual_codes().len(), 4, "Should have 4 codes before removal");

        // Execute: Remove code_def_1
        let result = codebook.remove_code_def(code_def_1);
        assert!(result.is_ok(), "Should successfully remove code_def_1");

        // Assert: Only codes from code_def_2 should remain
        let remaining_codes = codebook.get_all_qual_codes();
        assert_eq!(remaining_codes.len(), 2, "Should have 2 codes after removing code_def_1");

        let remaining_ids: Vec<QualCodeId> = remaining_codes.iter().map(|c| c.id).collect();
        assert!(!remaining_ids.contains(&code_1a), "code_1a should be removed");
        assert!(!remaining_ids.contains(&code_1b), "code_1b should be removed");
        assert!(remaining_ids.contains(&code_2a), "code_2a should remain");
        assert!(remaining_ids.contains(&code_2b), "code_2b should remain");

        // Assert: code_def_1 should be gone from the codebook
        assert!(codebook.code_def(code_def_1).is_none(), "code_def_1 should not exist");
        assert!(codebook.code_def(code_def_2).is_some(), "code_def_2 should still exist");
    }

    #[test]
    fn test_remove_code_def_with_no_applied_codes() {
        // Setup: Create a code def but never apply it
        let mut codebook = create_test_codebook();
        let code_def = codebook.create_code_def("Unused".to_string(), 1, None);

        assert_eq!(codebook.get_all_qual_codes().len(), 0, "Should have 0 codes initially");

        // Execute: Remove the unused code def
        let result = codebook.remove_code_def(code_def);

        // Assert: Should succeed
        assert!(result.is_ok(), "Should successfully remove unused code def");
        assert!(codebook.code_def(code_def).is_none(), "Code def should be removed");
        assert_eq!(codebook.get_all_qual_codes().len(), 0, "Should still have 0 codes");
    }

    #[test]
    fn test_remove_nonexistent_code_def_returns_error() {
        // Setup: Empty codebook
        let mut codebook = create_test_codebook();
        let fake_id = CodeDefId(Uuid::new_v4());

        // Execute: Try to remove a code def that doesn't exist
        let result = codebook.remove_code_def(fake_id);

        // Assert: Should return error
        assert!(result.is_err(), "Should return error for nonexistent code def");
        match result {
            Err(CodeBookError::CodeDefNotFound(id)) => {
                assert_eq!(id, fake_id, "Error should contain the correct ID");
            }
            _ => panic!("Expected CodeDefNotFound error"),
        }
    }

    #[test]
    fn test_remove_code_def_across_multiple_files() {
        // Setup: Apply the same code def to blocks in multiple files
        let mut codebook = create_test_codebook();
        let file_a = create_test_file("a.txt", 2);
        let file_b = create_test_file("b.txt", 2);
        let block_map = build_block_map(&[&file_a, &file_b]);

        let code_def = codebook.create_code_def("SharedCode".to_string(), 1, None);

        // Apply codes to both files
        let blocks_a = file_a.blocks().unwrap();
        let blocks_b = file_b.blocks().unwrap();
        apply_test_code(&mut codebook, blocks_a[0].id, code_def, "a1");
        apply_test_code(&mut codebook, blocks_a[1].id, code_def, "a2");
        apply_test_code(&mut codebook, blocks_b[0].id, code_def, "b1");
        apply_test_code(&mut codebook, blocks_b[1].id, code_def, "b2");

        assert_eq!(codebook.get_all_qual_codes().len(), 4, "Should have 4 codes before removal");

        // Execute: Remove the code def
        let result = codebook.remove_code_def(code_def);
        assert!(result.is_ok(), "Should successfully remove code def");

        // Assert: All codes removed, both files affected
        assert_eq!(codebook.get_all_qual_codes().len(), 0, "All codes should be removed");

        let codes_a: Vec<&QualCode> = codebook.get_codes_for_file(file_a.id, &block_map).collect();
        let codes_b: Vec<&QualCode> = codebook.get_codes_for_file(file_b.id, &block_map).collect();

        assert_eq!(codes_a.len(), 0, "File A should have 0 codes");
        assert_eq!(codes_b.len(), 0, "File B should have 0 codes");
    }

}
// ===== Tests for theme operations =====
mod theme_operations {
    use super::*;

    #[test]
    fn test_remove_theme_moves_codes_to_top_level() {
        // Setup: Create theme with codes, verify removal moves codes to top-level
        let mut codebook = create_test_codebook();

        let theme_id = codebook.create_theme("Theme1".to_string(), 1);
        let _code_def_1 = codebook.create_code_def("Code1".to_string(), 1, Some(theme_id));
        let _code_def_2 = codebook.create_code_def("Code2".to_string(), 2, Some(theme_id));
        let _code_def_3 = codebook.create_code_def("TopLevel".to_string(), 3, None);

        // Verify setup
        assert_eq!(codebook.get_codes_in_theme(theme_id).count(), 2, "Theme should have 2 codes");
        assert_eq!(codebook.get_top_level_codes().count(), 1, "Should have 1 top-level code");

        // Execute: Remove the theme
        let result = codebook.remove_theme(theme_id);
        assert!(result.is_ok(), "Should successfully remove theme");

        // Assert: All codes should now be top-level
        assert_eq!(codebook.get_top_level_codes().count(), 3, "All 3 codes should be top-level");
        assert_eq!(codebook.get_codes_in_theme(theme_id).count(), 0, "Removed theme should have 0 codes");

        // Verify theme is gone
        assert!(codebook.theme(theme_id).is_none(), "Theme should not exist");
    }

    #[test]
    fn test_move_code_to_theme_and_filtering() {
        // Setup: Create codes and themes, move codes between them
        let mut codebook = create_test_codebook();

        let theme_a = codebook.create_theme("ThemeA".to_string(), 1);
        let theme_b = codebook.create_theme("ThemeB".to_string(), 2);

        let code_1 = codebook.create_code_def("Code1".to_string(), 1, None);
        let code_2 = codebook.create_code_def("Code2".to_string(), 2, None);
        let code_3 = codebook.create_code_def("Code3".to_string(), 3, None);

        // All codes start as top-level
        assert_eq!(codebook.get_top_level_codes().count(), 3, "Should have 3 top-level codes");

        // Execute: Move codes to themes
        codebook.move_code_to_theme(code_1, theme_a);
        codebook.move_code_to_theme(code_2, theme_a);
        codebook.move_code_to_theme(code_3, theme_b);

        // Assert: Codes are in correct themes
        let theme_a_codes: Vec<CodeDefId> = codebook.get_codes_in_theme(theme_a)
            .map(|c| c.id)
            .collect();
        let theme_b_codes: Vec<CodeDefId> = codebook.get_codes_in_theme(theme_b)
            .map(|c| c.id)
            .collect();

        assert_eq!(theme_a_codes.len(), 2, "Theme A should have 2 codes");
        assert!(theme_a_codes.contains(&code_1), "Theme A should contain code_1");
        assert!(theme_a_codes.contains(&code_2), "Theme A should contain code_2");

        assert_eq!(theme_b_codes.len(), 1, "Theme B should have 1 code");
        assert!(theme_b_codes.contains(&code_3), "Theme B should contain code_3");

        assert_eq!(codebook.get_top_level_codes().count(), 0, "Should have 0 top-level codes");
    }

    #[test]
    fn test_remove_code_from_theme_returns_to_top_level() {
        // Setup: Create theme with codes
        let mut codebook = create_test_codebook();

        let theme_id = codebook.create_theme("Theme1".to_string(), 1);
        let code_1 = codebook.create_code_def("Code1".to_string(), 1, Some(theme_id));
        let _code_2 = codebook.create_code_def("Code2".to_string(), 2, Some(theme_id));

        assert_eq!(codebook.get_codes_in_theme(theme_id).count(), 2, "Theme should have 2 codes");
        assert_eq!(codebook.get_top_level_codes().count(), 0, "Should have 0 top-level codes");

        // Execute: Remove code_1 from theme
        let result = codebook.remove_code_from_theme(code_1);
        assert!(result.is_ok(), "Should successfully remove code from theme");

        // Assert: code_1 is now top-level, code_2 still in theme
        assert_eq!(codebook.get_codes_in_theme(theme_id).count(), 1, "Theme should have 1 code");
        assert_eq!(codebook.get_top_level_codes().count(), 1, "Should have 1 top-level code");

        let top_level_codes: Vec<CodeDefId> = codebook.get_top_level_codes().map(|c| c.id).collect();
        assert!(top_level_codes.contains(&code_1), "code_1 should be top-level");
    }

    #[test]
    fn test_move_code_between_themes() {
        // Setup: Create 2 themes and a code in theme A
        let mut codebook = create_test_codebook();

        let theme_a = codebook.create_theme("ThemeA".to_string(), 1);
        let theme_b = codebook.create_theme("ThemeB".to_string(), 2);
        let code_id = codebook.create_code_def("Code1".to_string(), 1, Some(theme_a));

        // Verify initial state
        assert_eq!(codebook.get_codes_in_theme(theme_a).count(), 1, "Theme A should have 1 code");
        assert_eq!(codebook.get_codes_in_theme(theme_b).count(), 0, "Theme B should have 0 codes");

        // Execute: Move code from theme A to theme B
        codebook.move_code_to_theme(code_id, theme_b);

        // Assert: Code moved from A to B
        assert_eq!(codebook.get_codes_in_theme(theme_a).count(), 0, "Theme A should have 0 codes");
        assert_eq!(codebook.get_codes_in_theme(theme_b).count(), 1, "Theme B should have 1 code");

        let theme_b_codes: Vec<CodeDefId> = codebook.get_codes_in_theme(theme_b).map(|c| c.id).collect();
        assert!(theme_b_codes.contains(&code_id), "Theme B should contain the code");
    }

    #[test]
    fn test_remove_nonexistent_theme_returns_error() {
        // Setup: Empty codebook
        let mut codebook = create_test_codebook();
        let fake_id = ThemeId(Uuid::new_v4());

        // Execute: Try to remove nonexistent theme
        let result = codebook.remove_theme(fake_id);

        // Assert: Should return error
        assert!(result.is_err(), "Should return error for nonexistent theme");
        match result {
            Err(CodeBookError::ThemeNotFound(id)) => {
                assert_eq!(id, fake_id, "Error should contain the correct ID");
            }
            _ => panic!("Expected ThemeNotFound error"),
        }
    }

    #[test]
    fn test_remove_code_from_theme_with_nonexistent_code() {
        // Setup: Empty codebook
        let mut codebook = create_test_codebook();
        let fake_id = CodeDefId(Uuid::new_v4());

        // Execute: Try to remove nonexistent code from theme
        let result = codebook.remove_code_from_theme(fake_id);

        // Assert: Should return error
        assert!(result.is_err(), "Should return error for nonexistent code");
        match result {
            Err(CodeBookError::CodeDefNotFound(id)) => {
                assert_eq!(id, fake_id, "Error should contain the correct ID");
            }
            _ => panic!("Expected CodeDefNotFound error"),
        }
    }

    #[test]
    fn test_theme_isolation_with_multiple_themes() {
        // Setup: Create 3 themes with codes in each
        let mut codebook = create_test_codebook();

        let theme_1 = codebook.create_theme("Theme1".to_string(), 1);
        let theme_2 = codebook.create_theme("Theme2".to_string(), 2);
        let theme_3 = codebook.create_theme("Theme3".to_string(), 3);

        let code_1a = codebook.create_code_def("Code1A".to_string(), 1, Some(theme_1));
        let code_1b = codebook.create_code_def("Code1B".to_string(), 2, Some(theme_1));
        let code_2a = codebook.create_code_def("Code2A".to_string(), 3, Some(theme_2));
        let code_3a = codebook.create_code_def("Code3A".to_string(), 4, Some(theme_3));
        let code_top = codebook.create_code_def("TopLevel".to_string(), 5, None);

        // Assert: Each theme has correct codes
        let t1_codes: Vec<CodeDefId> = codebook.get_codes_in_theme(theme_1).map(|c| c.id).collect();
        let t2_codes: Vec<CodeDefId> = codebook.get_codes_in_theme(theme_2).map(|c| c.id).collect();
        let t3_codes: Vec<CodeDefId> = codebook.get_codes_in_theme(theme_3).map(|c| c.id).collect();
        let top_codes: Vec<CodeDefId> = codebook.get_top_level_codes().map(|c| c.id).collect();

        assert_eq!(t1_codes.len(), 2, "Theme 1 should have 2 codes");
        assert!(t1_codes.contains(&code_1a) && t1_codes.contains(&code_1b));

        assert_eq!(t2_codes.len(), 1, "Theme 2 should have 1 code");
        assert!(t2_codes.contains(&code_2a));

        assert_eq!(t3_codes.len(), 1, "Theme 3 should have 1 code");
        assert!(t3_codes.contains(&code_3a));

        assert_eq!(top_codes.len(), 1, "Should have 1 top-level code");
        assert!(top_codes.contains(&code_top));

        // Execute: Remove theme 2
        codebook.remove_theme(theme_2).unwrap();

        // Assert: Theme 2's code moved to top-level, others unaffected
        assert_eq!(codebook.get_codes_in_theme(theme_1).count(), 2, "Theme 1 still has 2 codes");
        assert_eq!(codebook.get_codes_in_theme(theme_3).count(), 1, "Theme 3 still has 1 code");
        assert_eq!(codebook.get_top_level_codes().count(), 2, "Should now have 2 top-level codes");
    }
}

// ===== Tests for index manipulation operations =====
mod code_index_changes {
    use super::*;

    #[test]
    fn test_move_code_def_to_beginning() {
        // Setup: Create 3 code defs, move last to beginning
        let mut codebook = create_test_codebook();

        let code_1 = codebook.create_code_def("Code1".to_string(), 1, None);
        let code_2 = codebook.create_code_def("Code2".to_string(), 2, None);
        let code_3 = codebook.create_code_def("Code3".to_string(), 3, None);

        // Execute: Move code_3 to index 0
        let result = codebook.move_code_def_to_index(code_3, 0);
        assert!(result.is_ok(), "Should successfully move to beginning");

        // Assert: Order should be [code_3, code_1, code_2]
        let order: Vec<CodeDefId> = codebook.get_all_code_defs().map(|c| c.id).collect();
        assert_eq!(order, vec![code_3, code_1, code_2], "code_3 should be first");
    }

    #[test]
    fn test_move_code_def_to_end() {
        // Setup: Create 3 code defs, move first to end
        let mut codebook = create_test_codebook();

        let code_1 = codebook.create_code_def("Code1".to_string(), 1, None);
        let code_2 = codebook.create_code_def("Code2".to_string(), 2, None);
        let code_3 = codebook.create_code_def("Code3".to_string(), 3, None);

        // Execute: Move code_1 to index 2 (last position)
        let result = codebook.move_code_def_to_index(code_1, 2);
        assert!(result.is_ok(), "Should successfully move to end");

        // Assert: Order should be [code_2, code_3, code_1]
        let order: Vec<CodeDefId> = codebook.get_all_code_defs().map(|c| c.id).collect();
        assert_eq!(order, vec![code_2, code_3, code_1], "code_1 should be last");
    }

    #[test]
    fn test_move_code_def_to_same_index() {
        // Setup: Create 3 code defs
        let mut codebook = create_test_codebook();

        let code_1 = codebook.create_code_def("Code1".to_string(), 1, None);
        let code_2 = codebook.create_code_def("Code2".to_string(), 2, None);
        let code_3 = codebook.create_code_def("Code3".to_string(), 3, None);

        // Execute: Move code_2 to its current position (index 1)
        let result = codebook.move_code_def_to_index(code_2, 1);
        assert!(result.is_ok(), "Should succeed even when moving to same index");

        // Assert: Order unchanged
        let order: Vec<CodeDefId> = codebook.get_all_code_defs().map(|c| c.id).collect();
        assert_eq!(order, vec![code_1, code_2, code_3], "Order should remain unchanged");
    }

    #[test]
    fn test_move_code_def_invalid_index() {
        // Setup: Create 2 code defs
        let mut codebook = create_test_codebook();

        let code_1 = codebook.create_code_def("Code1".to_string(), 1, None);
        let _code_2 = codebook.create_code_def("Code2".to_string(), 2, None);

        // Execute: Try to move to index 5 (too large)
        let result = codebook.move_code_def_to_index(code_1, 5);

        // Assert: Should return error
        assert!(result.is_err(), "Should return error for invalid index");
        match result {
            Err(CodeBookError::InvalidIndex { provided, max }) => {
                assert_eq!(provided, 5);
                assert_eq!(max, 1); // max index for 2 items is 1
            }
            _ => panic!("Expected InvalidIndex error"),
        }
    }

    #[test]
    fn test_swap_code_defs() {
        // Setup: Create 4 code defs
        let mut codebook = create_test_codebook();

        let code_1 = codebook.create_code_def("Code1".to_string(), 1, None);
        let code_2 = codebook.create_code_def("Code2".to_string(), 2, None);
        let code_3 = codebook.create_code_def("Code3".to_string(), 3, None);
        let code_4 = codebook.create_code_def("Code4".to_string(), 4, None);

        // Execute: Swap indices 1 and 3
        let result = codebook.swap_code_defs(1, 3);
        assert!(result.is_ok(), "Should successfully swap");

        // Assert: Order should be [code_1, code_4, code_3, code_2]
        let order: Vec<CodeDefId> = codebook.get_all_code_defs().map(|c| c.id).collect();
        assert_eq!(order, vec![code_1, code_4, code_3, code_2], "code_2 and code_4 should be swapped");
    }

    #[test]
    fn test_swap_code_defs_invalid_index() {
        // Setup: Create 2 code defs
        let mut codebook = create_test_codebook();

        codebook.create_code_def("Code1".to_string(), 1, None);
        codebook.create_code_def("Code2".to_string(), 2, None);

        // Execute: Try to swap with invalid index
        let result = codebook.swap_code_defs(0, 5);

        // Assert: Should return error
        assert!(result.is_err(), "Should return error for invalid index");
    }

    #[test]
    fn test_move_theme_to_middle() {
        // Setup: Create 4 themes, move first to middle
        let mut codebook = create_test_codebook();

        let theme_1 = codebook.create_theme("Theme1".to_string(), 1);
        let theme_2 = codebook.create_theme("Theme2".to_string(), 2);
        let theme_3 = codebook.create_theme("Theme3".to_string(), 3);
        let theme_4 = codebook.create_theme("Theme4".to_string(), 4);

        // Execute: Move theme_1 to index 2
        let result = codebook.move_theme_to_index(theme_1, 2);
        assert!(result.is_ok(), "Should successfully move to middle");

        // Assert: Order should be [theme_2, theme_3, theme_1, theme_4]
        let order: Vec<ThemeId> = codebook.get_all_themes().map(|t| t.id).collect();
        assert_eq!(order, vec![theme_2, theme_3, theme_1, theme_4], "theme_1 should be at index 2");
    }

    #[test]
    fn test_swap_themes() {
        // Setup: Create 3 themes
        let mut codebook = create_test_codebook();

        let theme_1 = codebook.create_theme("Theme1".to_string(), 1);
        let theme_2 = codebook.create_theme("Theme2".to_string(), 2);
        let theme_3 = codebook.create_theme("Theme3".to_string(), 3);

        // Execute: Swap first and last
        let result = codebook.swap_themes(0, 2);
        assert!(result.is_ok(), "Should successfully swap");

        // Assert: Order should be [theme_3, theme_2, theme_1]
        let order: Vec<ThemeId> = codebook.get_all_themes().map(|t| t.id).collect();
        assert_eq!(order, vec![theme_3, theme_2, theme_1], "First and last should be swapped");
    }

    #[test]
    fn test_move_theme_nonexistent_theme() {
        // Setup: Empty codebook
        let mut codebook = create_test_codebook();
        let fake_id = ThemeId(Uuid::new_v4());

        // Execute: Try to move nonexistent theme
        let result = codebook.move_theme_to_index(fake_id, 0);

        // Assert: Should return error
        assert!(result.is_err(), "Should return error for nonexistent theme");
        match result {
            Err(CodeBookError::ThemeNotFound(id)) => {
                assert_eq!(id, fake_id);
            }
            _ => panic!("Expected ThemeNotFound error"),
        }
    }

    #[test]
    fn test_move_file_maintains_order() {
        // Setup: Create FileList with 3 files
        let mut file_list = FileList::new();

        let file_1 = file_list.add_file("file1.txt".to_string(), FileType::PlainText);
        let file_2 = file_list.add_file("file2.txt".to_string(), FileType::PlainText);
        let file_3 = file_list.add_file("file3.txt".to_string(), FileType::PlainText);

        // Execute: Move file_3 to beginning
        let result = file_list.move_file_to_index(file_3, 0);
        assert!(result.is_ok(), "Should successfully move file");

        // Assert: Order should be [file_3, file_1, file_2]
        let order: Vec<FileId> = file_list.get_all_files().map(|f| f.id).collect();
        assert_eq!(order, vec![file_3, file_1, file_2], "file_3 should be first");
    }

    #[test]
    fn test_swap_files() {
        // Setup: Create FileList with 3 files
        let mut file_list = FileList::new();

        let file_1 = file_list.add_file("file1.txt".to_string(), FileType::PlainText);
        let file_2 = file_list.add_file("file2.txt".to_string(), FileType::PlainText);
        let file_3 = file_list.add_file("file3.txt".to_string(), FileType::PlainText);

        // Execute: Swap indices 0 and 2
        let result = file_list.swap_files(0, 2);
        assert!(result.is_ok(), "Should successfully swap files");

        // Assert: Order should be [file_3, file_2, file_1]
        let order: Vec<FileId> = file_list.get_all_files().map(|f| f.id).collect();
        assert_eq!(order, vec![file_3, file_2, file_1], "First and last files should be swapped");
    }

    #[test]
    fn test_move_file_invalid_index() {
        // Setup: Create FileList with 2 files
        let mut file_list = FileList::new();

        let file_1 = file_list.add_file("file1.txt".to_string(), FileType::PlainText);
        file_list.add_file("file2.txt".to_string(), FileType::PlainText);

        // Execute: Try to move to invalid index
        let result = file_list.move_file_to_index(file_1, 10);

        // Assert: Should return error
        assert!(result.is_err(), "Should return error for invalid index");
        match result {
            Err(FileListError::InvalidIndex { provided, max }) => {
                assert_eq!(provided, 10);
                assert_eq!(max, 1);
            }
            _ => panic!("Expected InvalidIndex error"),
        }
    }

    #[test]
    fn test_swap_files_same_index() {
        // Setup: Create FileList with 2 files
        let mut file_list = FileList::new();

        let file_1 = file_list.add_file("file1.txt".to_string(), FileType::PlainText);
        let file_2 = file_list.add_file("file2.txt".to_string(), FileType::PlainText);

        // Execute: Swap same index with itself
        let result = file_list.swap_files(0, 0);
        assert!(result.is_ok(), "Should succeed even swapping with self");

        // Assert: Order unchanged
        let order: Vec<FileId> = file_list.get_all_files().map(|f| f.id).collect();
        assert_eq!(order, vec![file_1, file_2], "Order should remain unchanged");
    }
    // ===== Tests for sort operations =====

    #[test]
    fn test_sort_code_defs_by_name() {
        // Setup: Create code defs in random order
        let mut codebook = create_test_codebook();

        let code_c = codebook.create_code_def("Charlie".to_string(), 1, None);
        let code_a = codebook.create_code_def("Alice".to_string(), 2, None);
        let code_b = codebook.create_code_def("Bob".to_string(), 3, None);

        // Verify initial order
        let initial_order: Vec<CodeDefId> = codebook.get_all_code_defs().map(|c| c.id).collect();
        assert_eq!(initial_order, vec![code_c, code_a, code_b], "Initial order should be insertion order");

        // Execute: Sort by name
        codebook.sort_code_defs_by_name();

        // Assert: Should be alphabetically sorted
        let sorted_order: Vec<CodeDefId> = codebook.get_all_code_defs().map(|c| c.id).collect();
        assert_eq!(sorted_order, vec![code_a, code_b, code_c], "Should be sorted alphabetically");

        // Verify IDs are still accessible and data intact
        assert!(codebook.code_def(code_a).is_some(), "code_a should still be accessible");
        assert_eq!(codebook.code_def(code_a).unwrap().name(), "Alice");
    }

    #[test]
    fn test_sort_themes_by_name() {
        // Setup: Create themes in random order
        let mut codebook = create_test_codebook();

        let theme_z = codebook.create_theme("Zebra".to_string(), 1);
        let theme_a = codebook.create_theme("Apple".to_string(), 2);
        let theme_m = codebook.create_theme("Mango".to_string(), 3);

        // Execute: Sort by name
        codebook.sort_themes_by_name();

        // Assert: Should be alphabetically sorted
        let sorted_order: Vec<ThemeId> = codebook.get_all_themes().map(|t| t.id).collect();
        assert_eq!(sorted_order, vec![theme_a, theme_m, theme_z], "Should be sorted alphabetically");

        // Verify themes are still accessible
        assert!(codebook.theme(theme_z).is_some(), "theme_z should still be accessible");
        assert_eq!(codebook.theme(theme_z).unwrap().name(), "Zebra");
    }

    #[test]
    fn test_sort_files_by_name() {
        // Setup: Create files in random order
        let mut file_list = FileList::new();

        let file_c = file_list.add_file("charlie.txt".to_string(), FileType::PlainText);
        let file_a = file_list.add_file("alice.txt".to_string(), FileType::PlainText);
        let file_b = file_list.add_file("bob.txt".to_string(), FileType::PlainText);

        // Execute: Sort by name
        file_list.sort_files_by_name();

        // Assert: Should be alphabetically sorted
        let sorted_order: Vec<FileId> = file_list.get_all_files().map(|f| f.id).collect();
        assert_eq!(sorted_order, vec![file_a, file_b, file_c], "Should be sorted alphabetically");
    }

    #[test]
    fn test_sort_code_defs_with_same_names() {
        // Setup: Create code defs with duplicate names
        let mut codebook = create_test_codebook();

        let _code_1 = codebook.create_code_def("Duplicate".to_string(), 1, None);
        let _code_2 = codebook.create_code_def("Duplicate".to_string(), 2, None);
        let code_3 = codebook.create_code_def("Unique".to_string(), 3, None);

        // Execute: Sort by name
        codebook.sort_code_defs_by_name();

        // Assert: Duplicates should be stable (maintain relative order) or at least not crash
        let sorted_order: Vec<CodeDefId> = codebook.get_all_code_defs().map(|c| c.id).collect();
        assert_eq!(sorted_order.len(), 3, "All code defs should still exist");

        // "Duplicate" entries should come before "Unique"
        let unique_index = sorted_order.iter().position(|&id| id == code_3).unwrap();
        assert_eq!(unique_index, 2, "Unique should be last");
    }

    #[test]
    fn test_sort_preserves_theme_associations() {
        // Setup: Create code defs with themes, then sort
        let mut codebook = create_test_codebook();

        let theme_id = codebook.create_theme("MyTheme".to_string(), 1);
        let code_z = codebook.create_code_def("Zebra".to_string(), 1, Some(theme_id));
        let code_a = codebook.create_code_def("Apple".to_string(), 2, Some(theme_id));

        // Execute: Sort code defs
        codebook.sort_code_defs_by_name();

        // Assert: Theme associations should be preserved
        assert_eq!(codebook.get_codes_in_theme(theme_id).count(), 2, "Theme should still have 2 codes");

        let theme_codes: Vec<CodeDefId> = codebook.get_codes_in_theme(theme_id).map(|c| c.id).collect();
        assert!(theme_codes.contains(&code_a), "Theme should still contain code_a");
        assert!(theme_codes.contains(&code_z), "Theme should still contain code_z");
    }

    #[test]
    fn test_sort_empty_codebook() {
        // Setup: Empty codebook
        let mut codebook = create_test_codebook();

        // Execute: Sort (should not panic)
        codebook.sort_code_defs_by_name();
        codebook.sort_themes_by_name();

        // Assert: Should remain empty
        assert_eq!(codebook.get_all_code_defs().count(), 0);
        assert_eq!(codebook.get_all_themes().count(), 0);
    }

    #[test]
    fn test_sort_files_with_different_extensions() {
        // Setup: Create files with different extensions
        let mut file_list = FileList::new();

        let file_txt = file_list.add_file("document.txt".to_string(), FileType::PlainText);
        let file_md = file_list.add_file("readme.md".to_string(), FileType::Markdown);
        let file_pdf = file_list.add_file("article.pdf".to_string(), FileType::Pdf);

        // Execute: Sort by name (path)
        file_list.sort_files_by_name();

        // Assert: Should be sorted by full path including extension
        let sorted_order: Vec<FileId> = file_list.get_all_files().map(|f| f.id).collect();
        assert_eq!(sorted_order, vec![file_pdf, file_txt, file_md], "Should be sorted alphabetically by full path");
    }
}



// ===== Tests for Highlight edge cases =====
mod highlight_edge_cases {
    use super::*;

    #[test]
    fn test_highlight_with_equal_start_and_end() {
        // Setup: Create highlight with start == end
        let block_id = BlockId(Uuid::new_v4());
        let highlight = Highlight::new(block_id, 5, 5);

        // Assert: Should be empty with zero length
        assert!(highlight.is_empty(), "Highlight with equal start and end should be empty");
        assert_eq!(highlight.len(), 0, "Length should be 0");
        assert_eq!(highlight.start(), 5, "Start should be 5");
        assert_eq!(highlight.end(), 5, "End should be 5");
    }

    #[test]
    fn test_highlight_swaps_when_start_greater_than_end() {
        // Setup: Create highlight with start > end
        let block_id = BlockId(Uuid::new_v4());
        let highlight = Highlight::new(block_id, 100, 50);

        // Assert: Should automatically swap so start <= end
        assert_eq!(highlight.start(), 50, "Start should be swapped to smaller value");
        assert_eq!(highlight.end(), 100, "End should be swapped to larger value");
        assert_eq!(highlight.len(), 50, "Length should be 50");
        assert!(!highlight.is_empty(), "Should not be empty");
    }

    #[test]
    fn test_highlight_with_large_values() {
        // Setup: Create highlight with large usize values
        let block_id = BlockId(Uuid::new_v4());
        let large_start = 1_000_000;
        let large_end = 2_000_000;
        let highlight = Highlight::new(block_id, large_start, large_end);

        // Assert: Should handle large values correctly
        assert_eq!(highlight.start(), large_start);
        assert_eq!(highlight.end(), large_end);
        assert_eq!(highlight.len(), 1_000_000);
    }

    #[test]
    fn test_highlight_with_zero_start() {
        // Setup: Create highlight starting at 0
        let block_id = BlockId(Uuid::new_v4());
        let highlight = Highlight::new(block_id, 0, 100);

        // Assert: Should work correctly with 0 start
        assert_eq!(highlight.start(), 0);
        assert_eq!(highlight.end(), 100);
        assert_eq!(highlight.len(), 100);
    }
    #[test]
    fn test_highlight_single_character() {
        // Setup: Create highlight for single character (length 1)
        let block_id = BlockId(Uuid::new_v4());
        let highlight = Highlight::new(block_id, 10, 11);

        // Assert: Should have length 1
        assert_eq!(highlight.len(), 1, "Single character highlight should have length 1");
        assert!(!highlight.is_empty(), "Should not be empty");
    }
}
// ===== Tests for file removal and code coordination =====
mod file_removal {
    use super::*;

    #[test]
    fn test_remove_file_leaves_codes_orphaned() {
        // Setup: Create file with blocks and apply codes
        let mut codebook = create_test_codebook();
        let mut file_list = FileList::new();

        let file_id = file_list.add_file("test.txt".to_string(), FileType::PlainText);
        let _file = file_list.file(file_id).unwrap();

        // Manually add blocks to file (simulating what infrastructure would do)
        let block = TextBlock::new(file_id, 0, "content".to_string());
        let block_id = block.id;

        let code_def_id = codebook.create_code_def("Code1".to_string(), 1, None);
        let highlight = Highlight::new(block_id, 0, 5);
        codebook.apply_code(code_def_id, highlight, "snip".to_string(), String::new(), String::new());

        // Verify code exists
        assert_eq!(codebook.get_all_qual_codes().len(), 1, "Should have 1 code before file removal");

        // Execute: Remove file WITHOUT removing codes first
        let result = file_list.remove_file(file_id);
        assert!(result.is_ok(), "File removal should succeed");

        // Assert: Codes remain (orphaned) - this documents intentional design
        // The application layer is responsible for coordinating code removal
        assert_eq!(codebook.get_all_qual_codes().len(), 1, "Codes remain after file removal (by design)");

        // NOTE: This documents that FileList and CodeBook are intentionally decoupled.
        // The application layer must coordinate: remove_codes_for_file() then remove_file()
    }

    #[test]
    fn test_proper_file_removal_workflow() {
        // Setup: Create file with blocks and apply codes
        let mut codebook = create_test_codebook();
        let mut file_list = FileList::new();

        let file_id = file_list.add_file("test.txt".to_string(), FileType::PlainText);

        // Create blocks and add them to a test file
        let mut test_file = create_test_file("test.txt", 2);
        test_file.id = file_id; // Use the real file_id

        let code_def_id = codebook.create_code_def("Code1".to_string(), 1, None);
        let blocks = test_file.blocks().unwrap();
        apply_test_code(&mut codebook, blocks[0].id, code_def_id, "snip1");
        apply_test_code(&mut codebook, blocks[1].id, code_def_id, "snip2");

        // Verify setup
        assert_eq!(codebook.get_all_qual_codes().len(), 2, "Should have 2 codes initially");

        // Execute: Proper workflow - remove codes THEN file
        let block_map = build_block_map(&[&test_file]);
        codebook.remove_codes_for_file(file_id, &block_map);
        file_list.remove_file(file_id).unwrap();

        // Assert: Both codes and file are gone
        assert_eq!(codebook.get_all_qual_codes().len(), 0, "All codes should be removed");
        assert!(file_list.file(file_id).is_none(), "File should be removed");
    }
}


// ===== Tests for DataState transitions =====

mod file_data_states {
    use super::*;

    #[test]
    fn test_file_starts_empty() {
        // Setup: Create new file
        let mut file_list = FileList::new();
        let file_id = file_list.add_file("new.txt".to_string(), FileType::PlainText);
        let file = file_list.file(file_id).unwrap();

        // Assert: Should start in Empty state
        assert!(file.blocks().is_none(), "New file should have no blocks (Empty state)");
    }

    #[test]
    fn test_set_data_state_to_loaded() {
        // Setup: Create file through FileList
        let mut file_list = FileList::new();
        let file_id = file_list.add_file("test.txt".to_string(), FileType::PlainText);

        let block1 = TextBlock::new(file_id, 0, "First block".to_string());
        let block2 = TextBlock::new(file_id, 1, "Second block".to_string());
        let blocks = vec![block1, block2];

        // Execute: Set to Loaded state
        let file = file_list.file_mut(file_id).unwrap();
        file.set_data_state(DataState::Loaded(blocks));

        // Assert: Can retrieve blocks
        let file = file_list.file(file_id).unwrap();
        let retrieved_blocks = file.blocks();
        assert!(retrieved_blocks.is_some(), "Should have blocks in Loaded state");
        assert_eq!(retrieved_blocks.unwrap().len(), 2, "Should have 2 blocks");
        assert_eq!(retrieved_blocks.unwrap()[0].content, "First block");
        assert_eq!(retrieved_blocks.unwrap()[1].content, "Second block");
    }

    #[test]
    fn test_set_data_state_to_modified() {
        // Setup: Create file with Loaded state
        let mut file_list = FileList::new();
        let file_id = file_list.add_file("test.txt".to_string(), FileType::PlainText);

        let block = TextBlock::new(file_id, 0, "Original".to_string());
        let file = file_list.file_mut(file_id).unwrap();
        file.set_data_state(DataState::Loaded(vec![block]));

        // Execute: Transition to Modified state
        let modified_block = TextBlock::new(file_id, 0, "Modified content".to_string());
        let file = file_list.file_mut(file_id).unwrap();
        file.set_data_state(DataState::Modified(vec![modified_block]));

        // Assert: Can retrieve modified blocks
        let file = file_list.file(file_id).unwrap();
        let retrieved = file.blocks();
        assert!(retrieved.is_some(), "Should have blocks in Modified state");
        assert_eq!(retrieved.unwrap()[0].content, "Modified content");
    }

    #[test]
    fn test_data_state_error_returns_none() {
        // Setup: Create file in Error state
        let mut file_list = FileList::new();
        let file_id = file_list.add_file("error.txt".to_string(), FileType::PlainText);

        let file = file_list.file_mut(file_id).unwrap();
        file.set_data_state(DataState::Error);

        // Assert: blocks() returns None for Error state
        let file = file_list.file(file_id).unwrap();
        assert!(file.blocks().is_none(), "Error state should return None for blocks");
    }

    #[test]
    fn test_data_state_empty_returns_none() {
        // Setup: Create file and explicitly set to Empty
        let mut file_list = FileList::new();
        let file_id = file_list.add_file("empty.txt".to_string(), FileType::PlainText);

        let file = file_list.file_mut(file_id).unwrap();
        file.set_data_state(DataState::Empty);

        // Assert: blocks() returns None for Empty state
        let file = file_list.file(file_id).unwrap();
        assert!(file.blocks().is_none(), "Empty state should return None for blocks");
    }

    #[test]
    fn test_transition_from_loaded_to_empty() {
        // Setup: File with blocks
        let mut file_list = FileList::new();
        let file_id = file_list.add_file("test.txt".to_string(), FileType::PlainText);

        let block = TextBlock::new(file_id, 0, "Content".to_string());
        let file = file_list.file_mut(file_id).unwrap();
        file.set_data_state(DataState::Loaded(vec![block]));

        let file = file_list.file(file_id).unwrap();
        assert!(file.blocks().is_some(), "Should have blocks initially");

        // Execute: Transition to Empty
        let file = file_list.file_mut(file_id).unwrap();
        file.set_data_state(DataState::Empty);

        // Assert: Blocks are now None
        let file = file_list.file(file_id).unwrap();
        assert!(file.blocks().is_none(), "Should have no blocks after Empty transition");
    }
}
