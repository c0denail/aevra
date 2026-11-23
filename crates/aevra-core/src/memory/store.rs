use super::*;
use crate::{db::Database, types::now};
use rusqlite::{params, OptionalExtension};
const DAY: i64 = 86_400_000;
impl Database {
    pub(crate) fn migrate_memory(connection: &rusqlite::Connection) -> Result<()> {
        connection.execute_batch("BEGIN IMMEDIATE;
ALTER TABLE conversations ADD COLUMN memory_disabled INTEGER NOT NULL DEFAULT 0;
CREATE TABLE memories(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE memory_blocks(key TEXT PRIMARY KEY);
CREATE TABLE memory_evidence(key TEXT NOT NULL,message_id TEXT NOT NULL,conversation_id TEXT NOT NULL,value TEXT NOT NULL,PRIMARY KEY(key,message_id));
CREATE TABLE memory_changes(batch_id TEXT NOT NULL,key TEXT NOT NULL,before_value TEXT,after_value TEXT NOT NULL,created_at INTEGER NOT NULL,PRIMARY KEY(batch_id,key));
CREATE TABLE memory_meta(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL);INSERT INTO memory_meta VALUES(1,0);
CREATE TABLE memory_jobs(message_id TEXT PRIMARY KEY,conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,source TEXT NOT NULL,revision INTEGER NOT NULL);
PRAGMA user_version=5;COMMIT;")?;
        Ok(())
    }
    pub fn memory_revision(&self) -> Result<i64> {
        Ok(self
            .connection
            .query_row("SELECT revision FROM memory_meta WHERE id=1", [], |r| {
                r.get(0)
            })?)
    }
    pub fn invalidate_memory(&self) -> Result<()> {
        self.connection
            .execute("UPDATE memory_meta SET revision=revision+1 WHERE id=1", [])?;
        Ok(())
    }
    pub fn memory_state(&self) -> Result<MemoryState> {
        self.connection.execute("DELETE FROM memory_evidence WHERE key IN(SELECT key FROM memories WHERE json_extract(value,'$.expiresAt')<=?1)",[now()])?;
        self.connection.execute(
            "DELETE FROM memories WHERE json_extract(value,'$.expiresAt')<=?1",
            [now()],
        )?;
        self.connection.execute(
            "DELETE FROM memory_changes WHERE created_at<?1",
            [now() - 10 * 60_000],
        )?;
        let mut s = self.connection.prepare("SELECT value FROM memories")?;
        let mut items = s
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|r| Ok(serde_json::from_str::<Memory>(&r?)?))
            .collect::<Result<Vec<_>>>()?;
        let mut b = self
            .connection
            .prepare("SELECT key FROM memory_blocks ORDER BY key")?;
        let blocked_keys = b
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        items.retain(|m| m.expires_at.is_none_or(|t| t > now()));
        for m in &mut items {
            m.auto_update_disabled = blocked_keys.contains(&m.key);
        }
        items.sort_by_key(|m| std::cmp::Reverse(m.updated_at));
        Ok(MemoryState {
            items,
            blocked_keys,
        })
    }
    pub fn enqueue_memory(&self, conversation: &str, message: &str, source: &str) -> Result<()> {
        if !self.settings()?.memory_enabled || self.get(conversation)?.memory_disabled {
            return Ok(());
        }
        self.connection.execute(
            "INSERT OR IGNORE INTO memory_jobs VALUES(?1,?2,?3,?4)",
            params![message, conversation, source, self.memory_revision()?],
        )?;
        Ok(())
    }
    pub fn next_memory_job(&self) -> Result<Option<(String, String, String, i64)>> {
        self.connection.execute(
            "DELETE FROM memory_jobs WHERE revision!=(SELECT revision FROM memory_meta WHERE id=1)",
            [],
        )?;
        Ok(self.connection.query_row("SELECT message_id,conversation_id,source,revision FROM memory_jobs ORDER BY rowid LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?)
    }
    pub fn finish_memory_job(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM memory_jobs WHERE message_id=?1", [id])?;
        Ok(())
    }
    pub fn conversation_memory(&self, id: &str, disabled: bool) -> Result<()> {
        self.get(id)?;
        self.connection.execute(
            "UPDATE conversations SET memory_disabled=?1 WHERE id=?2",
            params![disabled, id],
        )?;
        self.invalidate_memory()
    }
    pub fn remember_candidates(
        &mut self,
        revision: i64,
        conversation: &str,
        message_id: &str,
        source: &str,
        candidates: Vec<Candidate>,
    ) -> Result<Option<MemoryNotice>> {
        if self.memory_revision()? != revision
            || !self.settings()?.memory_enabled
            || self.get(conversation)?.memory_disabled
        {
            return Ok(None);
        }
        let exists:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1 AND conversation_id=?2 AND role='user')",params![message_id,conversation],|r|r.get(0))?;
        if !exists {
            return Ok(None);
        }
        let batch = uuid::Uuid::new_v4().to_string();
        let time = now();
        let mut count = 0;
        let mut seen = std::collections::HashSet::new();
        let tx = self.connection.transaction()?;
        for c in candidates
            .into_iter()
            .filter(|c| valid_candidate(c, source))
        {
            let c = normalize_candidate(c, source);
            if !seen.insert(c.key.clone())
                || tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM memory_blocks WHERE key=?1)",
                    [&c.key],
                    |r| r.get::<_, bool>(0),
                )?
            {
                continue;
            }
            let before: Option<String> = tx
                .query_row("SELECT value FROM memories WHERE key=?1", [&c.key], |r| {
                    r.get(0)
                })
                .optional()?;
            let previous = before
                .as_deref()
                .map(serde_json::from_str::<Memory>)
                .transpose()?;
            if previous.as_ref().is_some_and(|m| {
                m.certainty != "pending" && m.certainty != "inferred" && c.certainty == "inferred"
            }) {
                continue;
            }
            tx.execute(
                "INSERT OR IGNORE INTO memory_evidence VALUES(?1,?2,?3,?4)",
                params![c.key, message_id, conversation, c.value.trim()],
            )?;
            let evidence=tx.query_row("SELECT count(DISTINCT conversation_id) FROM memory_evidence WHERE key=?1 AND value=?2",params![c.key,c.value.trim()],|r|r.get::<_,i64>(0))? as usize;
            // Do not replace a settled inference with a single contrary observation.
            if previous
                .as_ref()
                .is_some_and(|m| m.certainty != "pending" && m.value != c.value.trim())
                && c.certainty == "inferred"
                && evidence < 3
            {
                continue;
            }
            let certainty = if c.certainty == "explicit" {
                "explicit"
            } else if evidence >= 3 {
                "inferred"
            } else {
                "pending"
            };
            let item = Memory {
                key: c.key.clone(),
                label: label(&c.key).unwrap().into(),
                value: c.value.trim().into(),
                certainty: certainty.into(),
                evidence_count: evidence,
                created_at: previous.as_ref().map(|m| m.created_at).unwrap_or(time),
                updated_at: time,
                last_used_at: previous.as_ref().and_then(|m| m.last_used_at),
                expires_at: if c.temporary
                    || matches!(c.key.as_str(), "active_project" | "project_goal")
                {
                    Some(time + 90 * DAY)
                } else if certainty == "pending" {
                    Some(time + 30 * DAY)
                } else {
                    None
                },
                source_quote: c.quote,
                source_conversation_id: conversation.into(),
                auto_update_disabled: false,
            };
            let after = serde_json::to_string(&item)?;
            tx.execute("INSERT INTO memories VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![c.key,after])?;
            if previous
                .as_ref()
                .is_none_or(|m| m.value != item.value || m.certainty != item.certainty)
            {
                tx.execute(
                    "INSERT INTO memory_changes VALUES(?1,?2,?3,?4,?5)",
                    params![batch, c.key, before, after, time],
                )?;
                count += 1;
            }
        }
        tx.execute("DELETE FROM memory_evidence WHERE rowid NOT IN (SELECT rowid FROM memory_evidence ORDER BY rowid DESC LIMIT 1000)",[])?;
        tx.execute(
            "DELETE FROM memory_changes WHERE created_at<?1",
            [time - 10 * 60_000],
        )?;
        tx.commit()?;
        Ok((count > 0).then_some(MemoryNotice {
            batch_id: batch,
            count,
        }))
    }
    pub fn edit_memory(&mut self, key: &str, value: &str) -> Result<()> {
        if label(key).is_none()
            || value.trim().is_empty()
            || value.chars().count() > 240
            || sensitive(value)
            || value.chars().any(char::is_control)
        {
            return Err(Error::Validation(
                "Geçerli, hassas veri içermeyen kısa bir hafıza değeri yazın.".into(),
            ));
        }
        let mut item = self
            .memory_state()?
            .items
            .into_iter()
            .find(|m| m.key == key)
            .ok_or_else(|| Error::Validation("Hafıza kaydı bulunamadı.".into()))?;
        item.value = value.trim().into();
        item.certainty = "edited".into();
        item.source_quote = "Elle düzenlendi".into();
        item.updated_at = now();
        item.expires_at = if matches!(key, "active_project" | "project_goal") {
            Some(now() + 90 * DAY)
        } else {
            None
        };
        item.auto_update_disabled = false;
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE memories SET value=?1 WHERE key=?2",
            params![serde_json::to_string(&item)?, key],
        )?;
        tx.execute("DELETE FROM memory_changes WHERE key=?1", [key])?;
        tx.execute("DELETE FROM memory_blocks WHERE key=?1", [key])?;
        tx.execute("DELETE FROM memory_evidence WHERE key=?1", [key])?;
        tx.execute("UPDATE memory_meta SET revision=revision+1", [])?;
        tx.commit()?;
        Ok(())
    }
    pub fn delete_memory(&mut self, key: &str) -> Result<()> {
        if label(key).is_none() {
            return Err(Error::Validation("Geçersiz hafıza alanı.".into()));
        }
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM memories WHERE key=?1", [key])?;
        tx.execute("DELETE FROM memory_evidence WHERE key=?1", [key])?;
        tx.execute("DELETE FROM memory_changes WHERE key=?1", [key])?;
        tx.execute("INSERT OR IGNORE INTO memory_blocks VALUES(?1)", [key])?;
        tx.execute("UPDATE memory_meta SET revision=revision+1", [])?;
        tx.commit()?;
        Ok(())
    }
    pub fn clear_memories(&mut self) -> Result<()> {
        let keys = self
            .memory_state()?
            .items
            .into_iter()
            .map(|m| m.key)
            .collect::<Vec<_>>();
        for key in keys {
            self.delete_memory(&key)?;
        }
        self.invalidate_memory()
    }
    pub fn allow_memory(&self, key: &str) -> Result<()> {
        if label(key).is_none() {
            return Err(Error::Validation("Geçersiz hafıza alanı.".into()));
        }
        self.connection
            .execute("DELETE FROM memory_blocks WHERE key=?1", [key])?;
        self.invalidate_memory()
    }
    pub fn undo_memories(&mut self, batch: &str) -> Result<usize> {
        if uuid::Uuid::parse_str(batch).is_err() {
            return Err(Error::Validation("Geçersiz güncelleme kimliği.".into()));
        }
        let mut s=self.connection.prepare("SELECT key,before_value,after_value FROM memory_changes WHERE batch_id=?1 AND created_at>=?2")?;
        let changes = s
            .query_map(params![batch, now() - 10 * 60_000], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(s);
        let tx = self.connection.transaction()?;
        let mut count = 0;
        for (key, before, after) in changes {
            let current: Option<String> = tx
                .query_row("SELECT value FROM memories WHERE key=?1", [&key], |r| {
                    r.get(0)
                })
                .optional()?;
            if current.as_deref() != Some(&after) {
                continue;
            }
            if let Some(before) = before {
                tx.execute(
                    "UPDATE memories SET value=?1 WHERE key=?2",
                    params![before, key],
                )?;
            } else {
                tx.execute("DELETE FROM memories WHERE key=?1", [&key])?;
            }
            tx.execute("DELETE FROM memory_evidence WHERE key=?1", [&key])?;
            tx.execute("DELETE FROM memory_changes WHERE key=?1", [&key])?;
            tx.execute("INSERT OR IGNORE INTO memory_blocks VALUES(?1)", [&key])?;
            count += 1;
        }
        tx.execute("UPDATE memory_meta SET revision=revision+1", [])?;
        tx.commit()?;
        Ok(count)
    }
}
