// Legacy modules (Phase 0 baseline)
mod commands;
pub mod models;
mod services;
mod utils;

// New architecture layers (Phase 1+)
pub mod agent_engine;
pub mod agent_tools;
mod application;
mod domain;
pub mod gate_integration;
mod infrastructure;
mod interfaces;
mod kernel;
pub mod knowledge;
pub mod llm;
pub mod mission;
pub mod review;
#[cfg(test)]
mod test_support;
pub mod writing_rules;
pub use services::{load_openai_search_settings, OpenAiSearchSettings};

use application::command_usecases::global_config::{export_skill, import_skill};
use commands::agent::{
    ai_openai_chat_completion, fetch_openai_models, get_openai_provider_settings,
    save_openai_provider_settings,
};
use commands::agent_engine::{agent_turn_cancel, agent_turn_resume, agent_turn_start};
use commands::agent_session::{
    agent_session_append_events, agent_session_create, agent_session_delete, agent_session_hydrate,
    agent_session_list, agent_session_load, agent_session_recover, agent_session_update_meta,
};
use commands::ai::{
    append_chapter_history_event, get_ai_proposal, get_chapter_history, list_ai_proposals,
    save_ai_proposal, update_proposal_status,
};
use commands::asset::{
    copy_asset, create_asset_file, create_asset_folder, delete_asset_path, get_assets_tree,
    list_assets, read_asset, read_asset_file, save_asset, save_asset_file, update_asset_file_title,
    update_asset_folder_title,
};
use commands::chapter::{
    create_chapter, move_chapter, read_chapter, save_chapter, save_chapter_markdown,
    set_chapter_word_goal, trash_chapter, update_chapter_metadata,
};
use commands::export::{export_book_single, export_chapter, export_tree_multi, export_volume};
use commands::global_config::{
    delete_skill, delete_worker, get_global_rules, list_skills, list_workers, save_global_rules,
    save_skill, save_worker,
};
use commands::import::{
    import_asset, import_chapter, import_manuscript, import_manuscript_into_volume,
};
use commands::inspiration::{
    inspiration_generate_metadata_variants, inspiration_session_create, inspiration_session_delete,
    inspiration_session_list, inspiration_session_load, inspiration_session_save_state,
    inspiration_session_update_meta, inspiration_turn_cancel, inspiration_turn_start,
};
use commands::knowledge_docs::{
    create_knowledge_document, create_knowledge_folder, delete_knowledge_entry, get_knowledge_tree,
    read_knowledge_document, save_knowledge_document,
};
use commands::mission::{
    mission_cancel, mission_contextpack_build, mission_contextpack_get_latest,
    mission_contextpack_rebuild_if_stale, mission_contextpack_status, mission_create,
    mission_get_status, mission_interrupt, mission_knowledge_apply, mission_knowledge_decide,
    mission_knowledge_get_latest, mission_knowledge_list, mission_knowledge_repropose,
    mission_knowledge_rollback, mission_layer1_get, mission_layer1_upsert, mission_list,
    mission_macro_create, mission_macro_get_state, mission_pause, mission_recover, mission_resume,
    mission_resume_with_config, mission_review_answer, mission_review_get_latest,
    mission_review_get_pending_decision, mission_review_list, mission_start,
};
use commands::project::{
    create_project, get_project_tree, open_project, scan_projects_directory, trash_project,
    update_project_metadata,
};
use commands::project_bootstrap::{
    get_project_bootstrap_status, resume_project_bootstrap, start_project_bootstrap,
};
use commands::recycle::{
    empty_recycle_bin, empty_recycled_projects, list_recycle_items, list_recycled_projects,
    permanently_delete_recycle_item, permanently_delete_recycled_project, restore_recycle_item,
    restore_recycled_project,
};
use commands::search_index::{search_index_cancel, search_index_rebuild, search_index_status};
use commands::versioning::{
    vc_get_current_head, vc_recover, vc_rollback_by_call_id, vc_rollback_by_revision,
};
use commands::volume::{create_volume, read_volume, trash_volume, update_volume};
use commands::writing_stats::{
    clear_writing_stats, end_writing_session, get_consecutive_days, get_month_stats,
    get_writing_stats, get_year_stats, record_words_written, start_writing_session,
    update_writing_session,
};
use interfaces::tauri::tool_invoke;

macro_rules! app_commands {
    () => {
        tauri::generate_handler![
            create_project,
            open_project,
            get_project_tree,
            update_project_metadata,
            scan_projects_directory,
            start_project_bootstrap,
            get_project_bootstrap_status,
            resume_project_bootstrap,
            trash_project,
            list_recycled_projects,
            restore_recycled_project,
            permanently_delete_recycled_project,
            empty_recycled_projects,
            create_chapter,
            read_chapter,
            save_chapter,
            update_chapter_metadata,
            set_chapter_word_goal,
            trash_chapter,
            move_chapter,
            save_chapter_markdown,
            create_volume,
            read_volume,
            update_volume,
            trash_volume,
            list_recycle_items,
            restore_recycle_item,
            permanently_delete_recycle_item,
            empty_recycle_bin,
            import_asset,
            import_manuscript,
            import_manuscript_into_volume,
            import_chapter,
            export_chapter,
            export_volume,
            export_book_single,
            export_tree_multi,
            save_ai_proposal,
            get_ai_proposal,
            update_proposal_status,
            append_chapter_history_event,
            get_chapter_history,
            list_ai_proposals,
            start_writing_session,
            update_writing_session,
            end_writing_session,
            record_words_written,
            get_writing_stats,
            get_month_stats,
            get_year_stats,
            get_consecutive_days,
            clear_writing_stats,
            list_assets,
            read_asset,
            save_asset,
            copy_asset,
            get_assets_tree,
            read_asset_file,
            save_asset_file,
            create_asset_folder,
            create_asset_file,
            update_asset_file_title,
            update_asset_folder_title,
            delete_asset_path,
            get_knowledge_tree,
            read_knowledge_document,
            save_knowledge_document,
            create_knowledge_folder,
            create_knowledge_document,
            delete_knowledge_entry,
            get_openai_provider_settings,
            save_openai_provider_settings,
            fetch_openai_models,
            ai_openai_chat_completion,
            inspiration_session_create,
            inspiration_session_load,
            inspiration_session_save_state,
            inspiration_session_list,
            inspiration_session_update_meta,
            inspiration_session_delete,
            inspiration_generate_metadata_variants,
            inspiration_turn_start,
            inspiration_turn_cancel,
            agent_session_create,
            agent_session_append_events,
            agent_session_load,
            agent_session_hydrate,
            agent_session_list,
            agent_session_update_meta,
            agent_session_recover,
            agent_session_delete,
            vc_get_current_head,
            vc_rollback_by_revision,
            vc_rollback_by_call_id,
            vc_recover,
            search_index_status,
            search_index_rebuild,
            search_index_cancel,
            tool_invoke,
            agent_turn_start,
            agent_turn_cancel,
            agent_turn_resume,
            mission_create,
            mission_list,
            mission_get_status,
            mission_start,
            mission_pause,
            mission_resume,
            mission_interrupt,
            mission_resume_with_config,
            mission_recover,
            mission_cancel,
            mission_review_get_latest,
            mission_review_list,
            mission_review_get_pending_decision,
            mission_review_answer,
            mission_knowledge_get_latest,
            mission_knowledge_decide,
            mission_knowledge_apply,
            mission_knowledge_rollback,
            mission_knowledge_repropose,
            mission_knowledge_list,
            mission_layer1_get,
            mission_layer1_upsert,
            mission_contextpack_get_latest,
            mission_contextpack_build,
            mission_contextpack_status,
            mission_contextpack_rebuild_if_stale,
            mission_macro_create,
            mission_macro_get_state,
            list_skills,
            save_skill,
            delete_skill,
            import_skill,
            export_skill,
            list_workers,
            save_worker,
            delete_worker,
            get_global_rules,
            save_global_rules,
        ]
    };
}

const SESSION_CLEANUP_INTERVAL_SECS: u64 = 600;
const SESSION_CLEANUP_TTL_SECS: u64 = 1800;

fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|_app| {
            tauri::async_runtime::spawn(async {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                    SESSION_CLEANUP_INTERVAL_SECS,
                ));
                let ttl = std::time::Duration::from_secs(SESSION_CLEANUP_TTL_SECS);

                loop {
                    interval.tick().await;
                    crate::agent_engine::session_state::global().cleanup_stale(ttl);
                }
            });
            Ok(())
        })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    build_app()
        .invoke_handler(app_commands!())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
