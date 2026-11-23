use super::*;
use std::collections::HashSet;

impl Database {
    pub fn settings(&self) -> Result<AppSettings> {
        let json: Option<String> = self
            .connection
            .query_row("SELECT value FROM preferences WHERE id=1", [], |r| r.get(0))
            .optional()?;
        let settings: AppSettings = json
            .map(|j| serde_json::from_str(&j))
            .transpose()?
            .unwrap_or_default();
        settings.validate()?;
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        settings.validate()?;
        self.connection.execute("INSERT INTO preferences(id,value) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value", [serde_json::to_string(settings)?])?;
        self.invalidate_memory()
    }

    pub fn archive(&self, id: &str, archived: bool) -> Result<()> {
        self.get(id)?;
        self.connection.execute(
            "UPDATE conversations SET archived=?1 WHERE id=?2",
            params![archived, id],
        )?;
        Ok(())
    }

    pub fn archive_all(&self) -> Result<()> {
        self.connection
            .execute("UPDATE conversations SET archived=1", [])?;
        Ok(())
    }

    /// Persist the submitted message before contacting either model or search.
    pub fn begin_turn(&mut self, id: &str, request_id: &str, text: &str) -> Result<()> {
        self.begin_turn_with_attachments(id, request_id, text, &[])
    }
    pub fn begin_turn_with_attachments(
        &mut self,
        id: &str,
        request_id: &str,
        text: &str,
        attachments: &[crate::attachments::Attachment],
    ) -> Result<()> {
        crate::attachments::validate_batch(attachments)?;
        self.get(id)?;
        let title = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(64)
            .collect::<String>();
        let tx = self.connection.transaction()?;
        tx.execute("INSERT INTO messages(id,conversation_id,role,content,created_at,sources,status,attachments) VALUES(?1,?2,'user',?3,?4,'[]','complete',?5)", params![request_id, id, text, now(),serde_json::to_string(attachments)?])?;
        tx.execute("UPDATE conversations SET archived=0,title=CASE WHEN title='Yeni konuşma' THEN ?1 ELSE title END,updated_at=?2 WHERE id=?3", params![title,now(),id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn finish_turn(
        &mut self,
        id: &str,
        answer: &str,
        sources: Vec<Source>,
        status: &str,
    ) -> Result<Message> {
        self.get(id)?;
        if !["complete", "interrupted", "failed", "limited"].contains(&status) {
            return Err(Error::Validation("Geçersiz yanıt durumu.".into()));
        }
        let message = Message {
            id: Uuid::new_v4().to_string(),
            conversation_id: id.into(),
            role: "assistant".into(),
            content: answer.into(),
            created_at: now(),
            sources,
            status: status.into(),
            attachments: vec![],
        };
        let tx = self.connection.transaction()?;
        tx.execute("INSERT INTO messages(id,conversation_id,role,content,created_at,sources,status) VALUES(?1,?2,'assistant',?3,?4,?5,?6)", params![message.id,id,answer,message.created_at,serde_json::to_string(&message.sources)?,status])?;
        tx.execute(
            "UPDATE conversations SET updated_at=?1 WHERE id=?2",
            params![message.created_at, id],
        )?;
        tx.commit()?;
        Ok(message)
    }

    pub fn clear_history(&mut self) -> Result<()> {
        self.connection.execute("DELETE FROM conversations", [])?;
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    pub fn counts(&self) -> Result<(usize, usize)> {
        Ok((
            self.connection
                .query_row("SELECT count(*) FROM conversations", [], |r| {
                    r.get::<_, i64>(0)
                })? as usize,
            self.connection
                .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?
                as usize,
        ))
    }

    pub fn export_history(&self) -> Result<HistoryExport> {
        let conversations = self
            .list()?
            .into_iter()
            .map(|conversation| {
                Ok(ConversationExport {
                    messages: self.messages(&conversation.id)?,
                    conversation,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(HistoryExport {
            format_version: 2,
            exported_at: now(),
            conversations,
        })
    }

    /// Merge a validated backup; existing conversations are never overwritten.
    pub fn import_history(&mut self, backup: HistoryExport) -> Result<usize> {
        if ![1, 2].contains(&backup.format_version) || backup.conversations.len() > 10_000 {
            return Err(Error::Validation("Yedek biçimi desteklenmiyor.".into()));
        }
        let mut ids = HashSet::new();
        let mut message_ids = HashSet::new();
        let mut total = 0;
        for item in &backup.conversations {
            let chat = &item.conversation;
            if Uuid::parse_str(&chat.id).is_err()
                || !ids.insert(chat.id.clone())
                || chat.title.chars().count() > 200
            {
                return Err(Error::Validation(
                    "Yedekte geçersiz konuşma verisi var.".into(),
                ));
            }
            total += item.messages.len();
            if total > 100_000 {
                return Err(Error::Validation("Yedek çok fazla mesaj içeriyor.".into()));
            }
            for message in &item.messages {
                crate::attachments::validate_batch(&message.attachments)?;
                if Uuid::parse_str(&message.id).is_err()
                    || !message_ids.insert(message.id.clone())
                    || message.conversation_id != chat.id
                    || !["user", "assistant"].contains(&message.role.as_str())
                    || !["complete", "interrupted", "failed", "limited"]
                        .contains(&message.status.as_str())
                    || message.content.len() > 4_500_000
                    || message.sources.len() > 5
                    || (message.role != "user" && !message.attachments.is_empty())
                {
                    return Err(Error::Validation(
                        "Yedekte geçersiz mesaj verisi var.".into(),
                    ));
                }
                for source in &message.sources {
                    crate::research::validate_url(&source.url)?;
                    if source.title.len() > 1000
                        || source.excerpt.len() > 10_000
                        || !(1..=5).contains(&source.id)
                    {
                        return Err(Error::Validation(
                            "Yedekte geçersiz kaynak verisi var.".into(),
                        ));
                    }
                }
            }
        }
        let mut count = 0;
        let tx = self.connection.transaction()?;
        for item in backup.conversations {
            let chat = item.conversation;
            if tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM conversations WHERE id=?1)",
                [&chat.id],
                |r| r.get::<_, bool>(0),
            )? {
                continue;
            }
            tx.execute("INSERT INTO conversations(id,title,mode,updated_at,archived,memory_disabled) VALUES(?1,?2,?3,?4,?5,?6)", params![chat.id,chat.title,chat.mode.as_str(),chat.updated_at,chat.archived,chat.memory_disabled])?;
            for message in item.messages {
                tx.execute("INSERT INTO messages(id,conversation_id,role,content,created_at,sources,status,attachments) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![message.id,chat.id,message.role,message.content,message.created_at,serde_json::to_string(&message.sources)?,message.status,serde_json::to_string(&message.attachments)?])?;
            }
            count += 1;
        }
        tx.commit()?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrates_v2_archives_and_interrupted_replies_then_saves_math_and_code() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v2.db");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE conversations(id TEXT PRIMARY KEY,title TEXT NOT NULL,mode TEXT NOT NULL CHECK(mode IN ('chat','translate','research')),updated_at INTEGER NOT NULL,archived INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE messages(seq INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT NOT NULL UNIQUE,conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,role TEXT NOT NULL,content TEXT NOT NULL,created_at INTEGER NOT NULL,sources TEXT NOT NULL DEFAULT '[]',status TEXT NOT NULL DEFAULT 'complete');
            INSERT INTO conversations VALUES('old','Arşiv','chat',1,1);
            INSERT INTO messages(id,conversation_id,role,content,created_at,status) VALUES('user','old','user','Soru',1,'complete'),('assistant','old','assistant','Yarım',1,'interrupted');
            PRAGMA user_version=2;").unwrap();
        drop(connection);
        let mut db = Database::open(&path).unwrap();
        assert!(db.get("old").unwrap().archived);
        assert_eq!(db.messages("old").unwrap()[1].status, "interrupted");
        for mode in [Mode::Math, Mode::Code] {
            let chat = db.create(mode).unwrap();
            db.save_turn(&chat.id, "Soru", "Yanıt", vec![]).unwrap();
            assert_eq!(db.get(&chat.id).unwrap().mode, mode);
        }
        db.delete("old").unwrap();
        assert_eq!(db.counts().unwrap(), (2, 4));
        assert_eq!(
            db.connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
    }
    #[test]
    fn pre_gpu_preferences_keep_their_values_and_default_to_auto() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"theme":"dark","maxTokens":512,"defaultMode":"math"}"#)
                .unwrap();
        assert_eq!(settings.compute_device, ComputeDevice::Auto);
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.max_tokens, 512);
        assert!(serde_json::from_str::<AppSettings>(r#"{"computeDevice":"unknown"}"#).is_err());
    }
    #[test]
    fn migrates_v1_history_without_losing_existing_messages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE conversations(id TEXT PRIMARY KEY,title TEXT NOT NULL,mode TEXT NOT NULL,updated_at INTEGER NOT NULL);
            CREATE TABLE messages(seq INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT NOT NULL UNIQUE,conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,role TEXT NOT NULL,content TEXT NOT NULL,created_at INTEGER NOT NULL,sources TEXT NOT NULL DEFAULT '[]');
            INSERT INTO conversations VALUES('old','Eski sohbet','chat',1);
            INSERT INTO messages(id,conversation_id,role,content,created_at) VALUES('message','old','user','Korunan metin',1);
            PRAGMA user_version=1;").unwrap();
        drop(connection);
        let db = Database::open(&path).unwrap();
        assert!(!db.get("old").unwrap().archived);
        assert_eq!(db.messages("old").unwrap()[0].content, "Korunan metin");
        assert_eq!(db.messages("old").unwrap()[0].status, "complete");
    }
    #[test]
    fn stopped_and_empty_responses_keep_user_history_after_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("history.db");
        let mut db = Database::open(&path).unwrap();
        let chat = db.create(Mode::Chat).unwrap();
        db.begin_turn(&chat.id, &Uuid::new_v4().to_string(), "Kaybolmayan mesaj")
            .unwrap();
        db.finish_turn(&chat.id, "Yarım yanıt", vec![], "interrupted")
            .unwrap();
        db.begin_turn(&chat.id, &Uuid::new_v4().to_string(), "Hemen durdur")
            .unwrap();
        db.finish_turn(&chat.id, "", vec![], "interrupted").unwrap();
        drop(db);
        let db = Database::open(&path).unwrap();
        let messages = db.messages(&chat.id).unwrap();
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].content, "Kaybolmayan mesaj");
        assert_eq!(messages[1].content, "Yarım yanıt");
        assert_eq!(messages[3].status, "interrupted");
    }
    #[test]
    fn settings_archives_and_backup_round_trip_without_overwrite() {
        let mut db = Database::open(Path::new(":memory:")).unwrap();
        let chat = db.create(Mode::Chat).unwrap();
        db.save_turn(&chat.id, "Merhaba", "Yanıt", vec![]).unwrap();
        db.archive(&chat.id, true).unwrap();
        let mut settings = db.settings().unwrap();
        settings.theme = "dark".into();
        db.save_settings(&settings).unwrap();
        assert_eq!(db.settings().unwrap().theme, "dark");
        let backup = db.export_history().unwrap();
        db.clear_history().unwrap();
        assert_eq!(db.counts().unwrap(), (0, 0));
        assert_eq!(db.import_history(backup).unwrap(), 1);
        assert!(db.get(&chat.id).unwrap().archived);
        assert_eq!(db.import_history(db.export_history().unwrap()).unwrap(), 0);
        assert_eq!(db.messages(&chat.id).unwrap().len(), 2);
        settings.temperature = f64::NAN;
        assert!(db.save_settings(&settings).is_err());
    }
}
