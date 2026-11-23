use super::*;
use crate::{db::Database, prompts, types::GenerateRequest};
use std::path::Path;
fn db() -> Database {
    Database::open(Path::new(":memory:")).unwrap()
}
#[test]
fn observed_feedback_is_not_mistaken_for_a_confirmed_preference() {
    let c = style_feedback("Daha kısa anlat.").remove(0);
    assert_eq!(c.certainty, "inferred");
    assert_eq!(
        normalize_candidate(c, "Daha kısa anlat.").value,
        "Kısa yanıtlar"
    );
    assert!(style_feedback("Bana kısa yanıt verme").is_empty());
    assert!(style_feedback("Kısa yanıt ne demek?").is_empty());
}
fn candidate(key: &str, value: &str, source: &str, certainty: &str) -> Candidate {
    Candidate {
        key: key.into(),
        value: value.into(),
        quote: source.into(),
        certainty: certainty.into(),
        temporary: false,
    }
}
fn save(
    db: &mut Database,
    key: &str,
    value: &str,
    source: &str,
    certainty: &str,
) -> Option<MemoryNotice> {
    let chat = db.create(Mode::Chat).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    db.begin_turn(&chat.id, &id, source).unwrap();
    db.remember_candidates(
        db.memory_revision().unwrap(),
        &chat.id,
        &id,
        source,
        vec![candidate(key, value, source, certainty)],
    )
    .unwrap()
}
#[test]
fn facts_update_deduplicate_persist_and_undo() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("memory.db");
    let mut d = Database::open(&path).unwrap();
    save(
        &mut d,
        "language_level_de",
        "A2",
        "Almanca seviyem A2",
        "explicit",
    );
    save(
        &mut d,
        "language_level_de",
        "A2",
        "Almanca seviyem A2",
        "explicit",
    );
    assert_eq!(d.memory_state().unwrap().items.len(), 1);
    let update = save(
        &mut d,
        "language_level_de",
        "B1",
        "Almanca seviyem artık B1",
        "explicit",
    )
    .unwrap();
    assert_eq!(d.memory_state().unwrap().items[0].value, "B1");
    assert_eq!(d.undo_memories(&update.batch_id).unwrap(), 1);
    assert_eq!(d.memory_state().unwrap().items[0].value, "A2");
    assert!(save(
        &mut d,
        "language_level_de",
        "C1",
        "Almanca seviyem C1",
        "explicit"
    )
    .is_none());
    drop(d);
    let d = Database::open(&path).unwrap();
    assert_eq!(d.memory_state().unwrap().items[0].value, "A2");
    assert!(d.memory_state().unwrap().items[0].auto_update_disabled);
}
#[test]
fn repetition_needs_three_conversations_and_canonicalizes_paraphrases() {
    let mut d = db();
    save(
        &mut d,
        "response_length",
        "kısa",
        "Daha kısa anlat",
        "explicit",
    );
    let m = d.memory_state().unwrap().items;
    assert_eq!(m[0].certainty, "pending");
    assert!(context(&m, "Merhaba", Mode::Chat).is_empty());
    save(
        &mut d,
        "response_length",
        "kısa tut",
        "Cevabı kısa tut lütfen",
        "inferred",
    );
    save(
        &mut d,
        "response_length",
        "kısa yanıt",
        "Bana kısa yanıt ver",
        "inferred",
    );
    let m = d.memory_state().unwrap().items;
    assert_eq!(m[0].certainty, "inferred");
    assert_eq!(m[0].evidence_count, 3);
    assert!(context(&m, "Merhaba", Mode::Chat).contains("Kısa yanıtlar"));
    // A single opposite request does not erase an established preference.
    save(&mut d, "response_length", "uzun", "Uzun anlat", "inferred");
    assert_eq!(d.memory_state().unwrap().items[0].value, "Kısa yanıtlar");
}
#[test]
fn forget_tombstones_edits_and_old_workers_cannot_restore_deleted_data() {
    let mut d = db();
    save(
        &mut d,
        "profession",
        "mühendisim",
        "Ben mühendisim",
        "explicit",
    );
    let revision = d.memory_revision().unwrap();
    d.delete_memory("profession").unwrap();
    assert!(d.memory_state().unwrap().items.is_empty());
    assert!(save(
        &mut d,
        "profession",
        "mühendisim",
        "Ben mühendisim",
        "explicit"
    )
    .is_none());
    let chat = d.create(Mode::Chat).unwrap();
    d.begin_turn(&chat.id, "late", "Ben doktorum").unwrap();
    assert!(d
        .remember_candidates(
            revision,
            &chat.id,
            "late",
            "Ben doktorum",
            vec![candidate(
                "profession",
                "doktorum",
                "Ben doktorum",
                "explicit"
            )]
        )
        .unwrap()
        .is_none());
    d.allow_memory("profession").unwrap();
    save(&mut d, "profession", "doktorum", "Ben doktorum", "explicit");
    d.edit_memory("profession", "Öğretmen").unwrap();
    let item = d.memory_state().unwrap().items.remove(0);
    assert_eq!(item.value, "Öğretmen");
    assert_eq!(item.certainty, "edited");
    assert!(d.edit_memory("profession", "API_KEY=secret").is_err());
    d.delete_memory("language_level_de").unwrap();
    assert!(save(
        &mut d,
        "learning_goal",
        "Almanca seviyem A2",
        "Almanca seviyem A2",
        "explicit"
    )
    .is_none());
}
#[test]
fn disabled_memory_and_memory_free_conversations_have_no_jobs_or_writes() {
    let mut d = db();
    let chat = d.create(Mode::Chat).unwrap();
    d.begin_turn(&chat.id, "request", "Almanca seviyem A2")
        .unwrap();
    d.conversation_memory(&chat.id, true).unwrap();
    assert!(d.get(&chat.id).unwrap().memory_disabled);
    d.enqueue_memory(&chat.id, "request", "Almanca seviyem A2")
        .unwrap();
    assert!(d.next_memory_job().unwrap().is_none());
    assert!(d
        .remember_candidates(
            d.memory_revision().unwrap(),
            &chat.id,
            "request",
            "Almanca seviyem A2",
            vec![candidate(
                "language_level_de",
                "A2",
                "Almanca seviyem A2",
                "explicit"
            )]
        )
        .unwrap()
        .is_none());
    d.conversation_memory(&chat.id, false).unwrap();
    d.enqueue_memory(&chat.id, "request", "Almanca seviyem A2")
        .unwrap();
    assert!(d.next_memory_job().unwrap().is_some());
    let s = AppSettings {
        memory_enabled: false,
        ..AppSettings::default()
    };
    d.save_settings(&s).unwrap();
    assert!(d.next_memory_job().unwrap().is_none());
    assert!(save(&mut d, "profession", "doktorum", "Ben doktorum", "explicit").is_none());
}
#[test]
fn memory_is_local_context_and_never_added_to_research_query() {
    let mut d = db();
    save(
        &mut d,
        "profession",
        "mühendisim",
        "Ben mühendisim",
        "explicit",
    );
    save(
        &mut d,
        "language_level_de",
        "A2",
        "Almanca seviyem A2",
        "explicit",
    );
    let items = d.memory_state().unwrap().items;
    assert!(!context(&items, "Almanca cümleyi açıkla", Mode::Chat).contains("mühendisim"));
    assert!(context(&items, "Almanca cümleyi açıkla", Mode::Chat).contains("A2"));
    assert!(context(&items, "Almanca cümle", Mode::Translate).is_empty());
    assert_eq!(
        crate::research_chat::safe_query("mühendisim A2", "Rust araştır", &[]),
        "Rust araştır"
    );
    let request = GenerateRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: "chat".into(),
        text: "Rust araştır".into(),
        target_language: None,
        attachment_ids: vec![],
    };
    let messages = prompts::build(Mode::Research, &request, &[], &[]).unwrap();
    assert!(!messages[0].content.contains("A2"));
    assert_eq!(
        forget_keys("Almanca seviyemi unut"),
        vec!["language_level_de"]
    );
    assert!(forget_keys("Almanca seviyemi unutma").is_empty());
}
#[test]
fn temporary_records_expire_and_backups_keep_only_conversation_memory_mode() {
    let mut d = db();
    save(
        &mut d,
        "active_project",
        "Aevra",
        "Aevra üzerinde çalışıyorum",
        "explicit",
    );
    assert!(d.memory_state().unwrap().items[0].expires_at.is_some());
    d.connection
        .execute(
            "UPDATE memories SET value=json_set(value,'$.expiresAt',0)",
            [],
        )
        .unwrap();
    assert!(d.memory_state().unwrap().items.is_empty());
    let chat = d.create(Mode::Chat).unwrap();
    d.conversation_memory(&chat.id, true).unwrap();
    let backup = d.export_history().unwrap();
    let mut other = db();
    other.import_history(backup).unwrap();
    assert!(other.get(&chat.id).unwrap().memory_disabled);
    assert!(other.memory_state().unwrap().items.is_empty());
}
