use crate::{types::*, Error, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;
mod extended;

pub struct Database {
    pub(crate) connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 5 {
            return Err(Error::Validation(
                "Veritabanı daha yeni bir Aevra sürümüne ait. Uygulamayı güncelleyin.".into(),
            ));
        }
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA secure_delete=ON;
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, mode TEXT NOT NULL CHECK(mode IN ('chat','translate','research')),
                updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS messages (
                seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE,
                conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
                role TEXT NOT NULL CHECK(role IN ('user','assistant')), content TEXT NOT NULL,
                created_at INTEGER NOT NULL, sources TEXT NOT NULL DEFAULT '[]');
            CREATE INDEX IF NOT EXISTS messages_conversation ON messages(conversation_id, seq);
            CREATE TABLE IF NOT EXISTS preferences (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);")?;
        if version < 2 {
            connection.execute_batch(
                "BEGIN IMMEDIATE;
                ALTER TABLE conversations ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE messages ADD COLUMN status TEXT NOT NULL DEFAULT 'complete';
                PRAGMA user_version=2; COMMIT;",
            )?;
        }
        if version < 3 {
            connection.execute_batch("PRAGMA foreign_keys=OFF;
                BEGIN IMMEDIATE;
                CREATE TABLE conversations_v3 (
                    id TEXT PRIMARY KEY, title TEXT NOT NULL,
                    mode TEXT NOT NULL CHECK(mode IN ('chat','translate','research','math','code')),
                    updated_at INTEGER NOT NULL, archived INTEGER NOT NULL DEFAULT 0);
                INSERT INTO conversations_v3 SELECT id,title,mode,updated_at,archived FROM conversations;
                DROP TABLE conversations;
                ALTER TABLE conversations_v3 RENAME TO conversations;
                PRAGMA user_version=3;
                COMMIT;
                PRAGMA foreign_keys=ON;")?;
            let violations: i64 = connection.query_row(
                "SELECT count(*) FROM pragma_foreign_key_check",
                [],
                |row| row.get(0),
            )?;
            if violations != 0 {
                return Err(Error::Validation(
                    "Veritabanı bağlantıları doğrulanamadı.".into(),
                ));
            }
        }
        if version < 4 {
            connection.execute_batch("BEGIN IMMEDIATE; ALTER TABLE messages ADD COLUMN attachments TEXT NOT NULL DEFAULT '[]'; PRAGMA user_version=4; COMMIT;")?;
        }
        if version < 5 {
            Self::migrate_memory(&connection)?;
        }
        Ok(Self { connection })
    }

    pub fn create(&self, mode: Mode) -> Result<Conversation> {
        let item = Conversation {
            id: Uuid::new_v4().to_string(),
            title: "Yeni konuşma".into(),
            mode,
            updated_at: now(),
            archived: false,
            memory_disabled: false,
        };
        self.connection.execute(
            "INSERT INTO conversations (id,title,mode,updated_at) VALUES (?1,?2,?3,?4)",
            params![item.id, item.title, mode.as_str(), item.updated_at],
        )?;
        Ok(item)
    }

    pub fn list(&self) -> Result<Vec<Conversation>> {
        let mut statement = self.connection.prepare("SELECT id,title,mode,updated_at,archived,memory_disabled FROM conversations ORDER BY updated_at DESC, rowid DESC")?;
        let items = statement
            .query_map([], read_conversation)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(items)
    }

    pub fn get(&self, id: &str) -> Result<Conversation> {
        self.connection
            .query_row(
                "SELECT id,title,mode,updated_at,archived,memory_disabled FROM conversations WHERE id=?1",
                [id],
                read_conversation,
            )
            .optional()?
            .ok_or_else(|| Error::Validation("Konuşma bulunamadı.".into()))
    }

    pub fn messages(&self, id: &str) -> Result<Vec<Message>> {
        self.get(id)?;
        let mut statement = self.connection.prepare("SELECT id,role,content,created_at,sources,status,attachments FROM messages WHERE conversation_id=?1 ORDER BY seq")?;
        let rows = statement.query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        rows.map(|row| {
            let (message_id, role, content, created_at, sources, status, attachments) = row?;
            Ok(Message {
                id: message_id,
                conversation_id: id.into(),
                role,
                content,
                created_at,
                sources: serde_json::from_str(&sources)?,
                status,
                attachments: serde_json::from_str(&attachments)?,
            })
        })
        .collect()
    }

    /// Store a complete turn atomically. A cancelled/failed generation leaves no half turn.
    pub fn save_turn(
        &mut self,
        id: &str,
        text: &str,
        answer: &str,
        sources: Vec<Source>,
    ) -> Result<Message> {
        self.get(id)?;
        let created_at = now();
        let output = Message {
            id: Uuid::new_v4().to_string(),
            conversation_id: id.into(),
            role: "assistant".into(),
            content: answer.into(),
            created_at,
            sources,
            status: "complete".into(),
            attachments: vec![],
        };
        let title = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(64)
            .collect::<String>();
        let transaction = self.connection.transaction()?;
        transaction.execute("INSERT INTO messages (id,conversation_id,role,content,created_at,sources) VALUES (?1,?2,'user',?3,?4,'[]')",
            params![Uuid::new_v4().to_string(), id, text, created_at])?;
        transaction.execute("INSERT INTO messages (id,conversation_id,role,content,created_at,sources) VALUES (?1,?2,'assistant',?3,?4,?5)",
            params![output.id, id, answer, created_at, serde_json::to_string(&output.sources)?])?;
        transaction.execute("UPDATE conversations SET title=CASE WHEN title='Yeni konuşma' THEN ?1 ELSE title END, updated_at=?2 WHERE id=?3", params![title, created_at, id])?;
        transaction.commit()?;
        Ok(output)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM conversations WHERE id=?1", [id])?;
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    pub fn has_source(&self, url: &str) -> Result<bool> {
        Ok(self.connection.query_row("SELECT EXISTS(SELECT 1 FROM messages, json_each(messages.sources) WHERE json_extract(json_each.value,'$.url')=?1)", [url], |row| row.get(0))?)
    }
}

fn read_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    let mode: String = row.get(2)?;
    let mode = match mode.as_str() {
        "chat" => Mode::Chat,
        "translate" => Mode::Translate,
        "research" => Mode::Research,
        "math" => Mode::Math,
        "code" => Mode::Code,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(Conversation {
        id: row.get(0)?,
        title: row.get(1)?,
        mode,
        updated_at: row.get(3)?,
        archived: row.get(4)?,
        memory_disabled: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_unicode_sources_and_deletes_with_cascade() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let mut db = Database::open(&path).unwrap();
        let chat = db.create(Mode::Research).unwrap();
        let sources = vec![Source {
            id: 1,
            title: "Kaynak".into(),
            url: "https://example.com/".into(),
            excerpt: "Kanıt".into(),
            accessed_at: now(),
        }];
        db.save_turn(&chat.id, "Türkiye'de araştırma 🧠", "Yanıt [1]", sources)
            .unwrap();
        drop(db);
        let db = Database::open(&path).unwrap();
        assert_eq!(db.list().unwrap()[0].title, "Türkiye'de araştırma 🧠");
        let messages = db.messages(&chat.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].sources[0].id, 1);
        db.delete(&chat.id).unwrap();
        assert!(db.list().unwrap().is_empty());
        let count: i64 = db
            .connection
            .query_row("SELECT count(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn invalid_conversation_does_not_create_orphan_messages() {
        let mut db = Database::open(Path::new(":memory:")).unwrap();
        assert!(db.save_turn("unknown", "hello", "answer", vec![]).is_err());
    }
}
