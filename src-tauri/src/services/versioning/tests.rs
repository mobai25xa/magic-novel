#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::services::versioning::recovery::current_lock_count;
    use crate::services::{
        RollbackByCallIdInput, RollbackByRevisionInput, VcActor, VcCommitInput, VcCommitPort,
        VersioningService,
    };

    fn setup_temp_project() -> PathBuf {
        let base = std::env::temp_dir().join(format!("magic_vc_test_{}", uuid::Uuid::new_v4()));
        let manuscripts = base.join("manuscripts").join("vol1");
        fs::create_dir_all(&manuscripts).unwrap();

        let chapter = serde_json::json!({
            "schema_version": 1,
            "id": "ch_001",
            "title": "t",
            "content": {"type":"doc","content":[]},
            "counts": {
                "text_length_no_whitespace": 0,
                "word_count": null,
                "algorithm_version": 1,
                "last_calculated_at": 0
            },
            "created_at": 0,
            "updated_at": 0
        });
        fs::write(
            manuscripts.join("ch_001.json"),
            serde_json::to_string_pretty(&chapter).unwrap(),
        )
        .unwrap();

        base
    }

    fn commit_once(
        svc: &VersioningService,
        project_path: &str,
        call_id: &str,
        expected_revision: i64,
        before_hash: &str,
        text: &str,
    ) -> String {
        let chapter = serde_json::json!({
            "schema_version": 1,
            "id": "ch_001",
            "title": "t",
            "content": {
                "type":"doc",
                "content":[{"type":"paragraph","attrs":{"id":"p1"},"content":[{"type":"text","text": text}]}]
            },
            "counts": {
                "text_length_no_whitespace": 0,
                "word_count": null,
                "algorithm_version": 1,
                "last_calculated_at": 0
            },
            "created_at": 0,
            "updated_at": 0
        });

        let out = svc
            .commit_with_occ(VcCommitInput {
                project_path: project_path.to_string(),
                entity_id: "chapter:vol1/ch_001.json".to_string(),
                expected_revision,
                call_id: call_id.to_string(),
                actor: VcActor::Agent,
                before_hash: before_hash.to_string(),
                after_json: chapter,
                patch_ops: vec![],
            })
            .unwrap();

        out.after_hash
    }

    #[test]
    fn occ_conflict_and_idempotent_call_id() {
        let project = setup_temp_project();
        let svc = VersioningService::new();
        let p = project.to_string_lossy().to_string();

        let h0 = svc
            .get_current_head(&p, "chapter:vol1/ch_001.json")
            .unwrap();
        assert_eq!(h0.revision, 0);

        let h1_hash = commit_once(&svc, &p, "call_a", 0, &h0.json_hash, "v1");

        let h1 = svc
            .get_current_head(&p, "chapter:vol1/ch_001.json")
            .unwrap();
        assert_eq!(h1.revision, 1);
        assert_eq!(h1.json_hash, h1_hash);

        let dup = svc
            .commit_with_occ(VcCommitInput {
                project_path: p.clone(),
                entity_id: "chapter:vol1/ch_001.json".to_string(),
                expected_revision: 0,
                call_id: "call_a".to_string(),
                actor: VcActor::Agent,
                before_hash: h0.json_hash.clone(),
                after_json: serde_json::json!({"type":"doc","content":[]}),
                patch_ops: vec![],
            })
            .unwrap();
        assert_eq!(dup.revision_after, 1);

        let conflict = svc.commit_with_occ(VcCommitInput {
            project_path: p.clone(),
            entity_id: "chapter:vol1/ch_001.json".to_string(),
            expected_revision: 0,
            call_id: "call_b".to_string(),
            actor: VcActor::Agent,
            before_hash: h0.json_hash.clone(),
            after_json: serde_json::json!({"type":"doc","content":[]}),
            patch_ops: vec![],
        });
        assert!(conflict.is_err());

        let err = format!("{}", conflict.unwrap_err());
        assert!(err.contains("E_VC_CONFLICT_REVISION"));
    }

    #[test]
    fn rollback_and_recover_flow() {
        let project = setup_temp_project();
        let svc = VersioningService::new();
        let p = project.to_string_lossy().to_string();

        let h0 = svc
            .get_current_head(&p, "chapter:vol1/ch_001.json")
            .unwrap();
        let h1_hash = commit_once(&svc, &p, "call_1", 0, &h0.json_hash, "r1");
        let _h2_hash = commit_once(&svc, &p, "call_2", 1, &h1_hash, "r2");

        let rb = svc
            .rollback_by_revision(RollbackByRevisionInput {
                project_path: p.clone(),
                entity_id: "chapter:vol1/ch_001.json".to_string(),
                target_revision: 1,
                call_id: "rb_1".to_string(),
                actor: VcActor::System,
                reason: Some("test".to_string()),
            })
            .unwrap();
        assert!(rb.revision_after >= 3);
        assert_eq!(rb.rolled_back_to_revision, 1);

        let rb2 = svc
            .rollback_by_call_id(RollbackByCallIdInput {
                project_path: p.clone(),
                target_call_id: "call_2".to_string(),
                call_id: "rb_2".to_string(),
                actor: VcActor::System,
                reason: None,
            })
            .unwrap();
        assert!(rb2.revision_after > rb.revision_after);

        let rec = svc.recover(&p).unwrap();
        assert!(rec.ok);

        let lock_count = current_lock_count(&p).unwrap();
        assert_eq!(lock_count, 0);
    }
}
