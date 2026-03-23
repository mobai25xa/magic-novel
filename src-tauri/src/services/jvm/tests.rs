#[cfg(test)]
mod tests {
    use crate::models::jvm::PatchOp;
    use crate::services::jvm::validator::validate_doc_integrity;
    use crate::services::jvm::{
        apply_patch_ops_to_doc, build_doc_from_markdown_blocks, ensure_doc_block_ids,
        generate_patch_ops, parse_markdown_to_blocks,
    };

    #[test]
    fn parser_extracts_block_hints() {
        let md = "<!-- @block:id=p1 type=paragraph -->\nHello\n\n# Title";
        let (blocks, diags) = parse_markdown_to_blocks(md).unwrap();
        assert!(diags.is_empty());
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_id.as_deref(), Some("p1"));
    }

    #[test]
    fn parser_dedupes_duplicate_block_ids_with_warning() {
        let md =
            "<!-- @block:id=p1 type=paragraph -->\nA\n\n<!-- @block:id=p1 type=paragraph -->\nB";
        let (blocks, diags) = parse_markdown_to_blocks(md).unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_id.as_deref(), Some("p1"));
        assert_ne!(blocks[1].block_id.as_deref(), Some("p1"));

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E_JVM_BLOCK_ID_DUP_DEDUPED");
        assert_eq!(diags[0].level, crate::models::jvm::DiagnosticLevel::Warn);
    }

    #[test]
    fn patcher_generates_update_for_changed_text() {
        let base = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "old"}]}
            ]
        });

        let md = "<!-- @block:id=p1 type=paragraph -->\nnew";
        let (md_blocks, _) = parse_markdown_to_blocks(md).unwrap();
        let (ops, diags, _summary) = generate_patch_ops(&base, &md_blocks);
        assert!(diags
            .iter()
            .all(|d| d.level != crate::models::jvm::DiagnosticLevel::Error));
        assert!(ops
            .iter()
            .any(|op| matches!(op, PatchOp::UpdateBlock { block_id, .. } if block_id == "p1")));
    }

    #[test]
    fn commit_apply_patch_updates_doc() {
        let base = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "old"}]}
            ]
        });

        let ops = vec![PatchOp::UpdateBlock {
            block_id: "p1".to_string(),
            before: None,
            after: serde_json::json!({
                "type": "paragraph",
                "attrs": {"id": "p1"},
                "content": [{"type": "text", "text": "new"}]
            }),
        }];

        let after = apply_patch_ops_to_doc(&base, &ops).unwrap();
        let text = after.to_string();
        assert!(text.contains("new"));
        assert!(!text.contains("old"));
    }

    #[test]
    fn patcher_preserves_order_for_consecutive_insert_blocks() {
        let base = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "base"}]}
            ]
        });

        let md = "<!-- @block:id=p1 type=paragraph -->\nbase\n\nfirst insert\n\nsecond insert";
        let (md_blocks, _) = parse_markdown_to_blocks(md).unwrap();
        let (ops, diags, _summary) = generate_patch_ops(&base, &md_blocks);

        assert!(diags
            .iter()
            .all(|d| d.level != crate::models::jvm::DiagnosticLevel::Error));

        let after = apply_patch_ops_to_doc(&base, &ops).unwrap();
        let content = after
            .get("content")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let texts: Vec<String> = content
            .iter()
            .map(|node| {
                node.get("content")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();

        assert_eq!(texts, vec!["base", "first insert", "second insert"]);
    }

    #[test]
    fn patcher_updates_without_appending_when_hints_are_preserved() {
        let base = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "old"}]},
                {"type": "paragraph", "attrs": {"id": "p2"}, "content": [{"type": "text", "text": "tail"}]}
            ]
        });

        let md = "<!-- @block:id=p1 type=paragraph -->\nnew\n\n<!-- @block:id=p2 type=paragraph -->\ntail";
        let (md_blocks, _) = parse_markdown_to_blocks(md).unwrap();
        let (ops, _, _) = generate_patch_ops(&base, &md_blocks);
        let after = apply_patch_ops_to_doc(&base, &ops).unwrap();
        let content = after.get("content").and_then(|v| v.as_array()).unwrap();

        assert_eq!(content.len(), 2);
        let texts: Vec<String> = content
            .iter()
            .map(|node| {
                node.get("content")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        assert_eq!(texts, vec!["new", "tail"]);
    }

    #[test]
    fn patcher_inserts_leading_block_at_document_start_when_anchor_is_missing() {
        let base = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "tail"}]}
            ]
        });

        let md = "lead\n\n<!-- @block:id=p1 type=paragraph -->\ntail";
        let (md_blocks, _) = parse_markdown_to_blocks(md).unwrap();
        let (ops, _, _) = generate_patch_ops(&base, &md_blocks);
        let after = apply_patch_ops_to_doc(&base, &ops).unwrap();
        let content = after.get("content").and_then(|v| v.as_array()).unwrap();

        let texts: Vec<String> = content
            .iter()
            .map(|node| {
                node.get("content")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();

        assert_eq!(texts, vec!["lead", "tail"]);
    }

    // ── Phase 1/2/3 regression tests ────────────────────────────────────

    fn extract_block_texts(doc: &serde_json::Value) -> Vec<String> {
        doc.get("content")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .map(|node| {
                node.get("content")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect()
    }

    fn all_blocks_have_ids(doc: &serde_json::Value) -> bool {
        doc.get("content")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter(|n| {
                let t = n.get("type").and_then(|v| v.as_str()).unwrap_or("");
                matches!(t, "heading" | "paragraph" | "blockquote")
            })
            .all(|n| {
                n.get("attrs")
                    .and_then(|a| a.get("id"))
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.is_empty())
            })
    }

    #[test]
    fn to_tiptap_block_always_assigns_id() {
        let md = "Hello world\n\n# Title\n\n> quote";
        let (blocks, _) = parse_markdown_to_blocks(md).unwrap();
        assert_eq!(blocks.len(), 3);

        for block in &blocks {
            let tiptap = crate::services::jvm::to_tiptap_block(block);
            let id = tiptap
                .get("attrs")
                .and_then(|a| a.get("id"))
                .and_then(|v| v.as_str());
            assert!(id.is_some_and(|s| !s.is_empty()), "block should have id");
        }
    }

    #[test]
    fn build_doc_all_blocks_have_ids() {
        let md = "Para one\n\nPara two\n\n# Heading";
        let (blocks, _) = parse_markdown_to_blocks(md).unwrap();
        let doc = build_doc_from_markdown_blocks(&blocks);
        assert!(all_blocks_have_ids(&doc));
    }

    #[test]
    fn consecutive_full_replace_does_not_duplicate_content() {
        // Simulates the core bug: consecutive edits without block hints
        // should NOT cause content to accumulate.
        let md_v1 = "Para one\n\nPara two";
        let (blocks_v1, _) = parse_markdown_to_blocks(md_v1).unwrap();
        let doc_v1 = build_doc_from_markdown_blocks(&blocks_v1);
        assert_eq!(extract_block_texts(&doc_v1), vec!["Para one", "Para two"]);
        assert!(all_blocks_have_ids(&doc_v1));

        let content_v1 = doc_v1.get("content").and_then(|v| v.as_array()).unwrap();
        let id_1 = content_v1[0]
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        let id_2 = content_v1[1]
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let exported_v1 = format!(
            "<!-- @block:id={id_1} type=paragraph -->\nPara one\n\n<!-- @block:id={id_2} type=paragraph -->\nPara two"
        );

        // Simulate str_replace: replace "Para one" with "Changed"
        let md_v2 = exported_v1.replace("Para one", "Changed");
        let (blocks_v2, _) = parse_markdown_to_blocks(&md_v2).unwrap();
        let doc_v2 = build_doc_from_markdown_blocks(&blocks_v2);

        assert_eq!(extract_block_texts(&doc_v2), vec!["Changed", "Para two"]);
        assert!(all_blocks_have_ids(&doc_v2));

        let exported_v2 = format!(
            "<!-- @block:id={id_1} type=paragraph -->\nChanged\n\n<!-- @block:id={id_2} type=paragraph -->\nPara two"
        );
        let md_v3 = exported_v2.replace("Changed", "Final");
        let (blocks_v3, _) = parse_markdown_to_blocks(&md_v3).unwrap();
        let doc_v3 = build_doc_from_markdown_blocks(&blocks_v3);

        assert_eq!(
            extract_block_texts(&doc_v3),
            vec!["Final", "Para two"],
            "content must not accumulate across consecutive edits"
        );
        assert!(all_blocks_have_ids(&doc_v3));
    }

    #[test]
    fn full_replace_without_hints_does_not_duplicate() {
        // Markdown with NO block hints at all (LLM MODE 2 scenario).
        let base = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "old A"}]},
                {"type": "paragraph", "attrs": {"id": "p2"}, "content": [{"type": "text", "text": "old B"}]}
            ]
        });

        let md_new = "new X\n\nnew Y\n\nnew Z";
        let (blocks, _) = parse_markdown_to_blocks(md_new).unwrap();
        let doc = build_doc_from_markdown_blocks(&blocks);

        let texts = extract_block_texts(&doc);
        assert_eq!(texts, vec!["new X", "new Y", "new Z"]);
        assert!(all_blocks_have_ids(&doc));

        // Verify via diff that old blocks would be deleted.
        let (ops, _, _) = generate_patch_ops(&base, &blocks);
        let has_delete = ops
            .iter()
            .any(|op| matches!(op, PatchOp::DeleteBlocks { .. }));
        assert!(
            has_delete,
            "old blocks should be marked for deletion in diff report"
        );
    }

    #[test]
    fn str_replace_preserves_unchanged_block_ids() {
        let exported = "<!-- @block:id=h1 type=heading -->\n# Title\n\n<!-- @block:id=p1 type=paragraph -->\nKeep me\n\n<!-- @block:id=p2 type=paragraph -->\nChange me";
        let replaced = exported.replace("Change me", "Changed");

        let (blocks, _) = parse_markdown_to_blocks(&replaced).unwrap();
        let new_doc = build_doc_from_markdown_blocks(&blocks);
        let content = new_doc.get("content").and_then(|v| v.as_array()).unwrap();

        // h1 and p1 should retain their original IDs.
        let h1_id = content[0]
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str());
        let p1_id = content[1]
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str());
        let p2_id = content[2]
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str());

        assert_eq!(h1_id, Some("h1"), "heading ID must be preserved");
        assert_eq!(
            p1_id,
            Some("p1"),
            "unchanged paragraph ID must be preserved"
        );
        assert_eq!(
            p2_id,
            Some("p2"),
            "changed paragraph ID must be preserved via hint"
        );
    }

    #[test]
    fn ensure_doc_block_ids_repairs_missing_ids() {
        let mut doc = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {}, "content": [{"type": "text", "text": "no id"}]},
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "has id"}]},
                {"type": "heading", "attrs": {"level": 1}, "content": [{"type": "text", "text": "heading no id"}]}
            ]
        });

        let repaired = ensure_doc_block_ids(&mut doc);
        assert_eq!(repaired, 2, "should repair 2 blocks missing IDs");
        assert!(all_blocks_have_ids(&doc));

        // The block that already had an ID should keep it.
        let content = doc.get("content").and_then(|v| v.as_array()).unwrap();
        let p1_id = content[1]
            .get("attrs")
            .and_then(|a| a.get("id"))
            .and_then(|v| v.as_str());
        assert_eq!(p1_id, Some("p1"));
    }

    #[test]
    fn validate_doc_integrity_catches_missing_ids() {
        let doc_ok = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": []}
            ]
        });
        let diags_ok = validate_doc_integrity(&doc_ok);
        assert!(
            diags_ok
                .iter()
                .all(|d| d.level != crate::models::DiagnosticLevel::Error),
            "valid doc should have no errors"
        );

        let doc_bad = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {}, "content": []}
            ]
        });
        let diags_bad = validate_doc_integrity(&doc_bad);
        assert!(
            diags_bad.iter().any(|d| d.code == "E_JVM_DOC_MISSING_ID"),
            "should detect missing block ID"
        );
    }

    #[test]
    fn validate_doc_integrity_catches_duplicate_ids() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "dup"}, "content": []},
                {"type": "paragraph", "attrs": {"id": "dup"}, "content": []}
            ]
        });
        let diags = validate_doc_integrity(&doc);
        assert!(
            diags.iter().any(|d| d.code == "E_JVM_DOC_DUPLICATE_ID"),
            "should detect duplicate IDs"
        );
    }

    #[test]
    fn apply_patch_ops_repairs_id_less_blocks_as_defense() {
        let base = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "keep"}]}
            ]
        });

        // Insert a block without an ID (should not happen, but defense-in-depth).
        let ops = vec![PatchOp::InsertBlocks {
            after_block_id: Some("p1".to_string()),
            blocks: vec![serde_json::json!({
                "type": "paragraph",
                "attrs": {},
                "content": [{"type": "text", "text": "no id block"}]
            })],
        }];

        let after = apply_patch_ops_to_doc(&base, &ops).unwrap();
        assert!(
            all_blocks_have_ids(&after),
            "apply_patch_ops_to_doc must repair blocks missing IDs"
        );
    }
}
