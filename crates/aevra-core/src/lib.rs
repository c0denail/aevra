pub mod attachments;
pub mod browser;
pub mod db;
pub mod math;
pub mod memory;
pub mod model;
pub mod pdf_export;
pub mod prompts;
pub mod research;
pub mod research_chat;
pub mod translation;
pub mod types;
#[cfg(windows)]
mod windows_job;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("Yerel veritabanı işlemi başarısız: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Dosya işlemi başarısız: {0}")]
    Io(#[from] std::io::Error),
    #[error("Veri biçimi okunamadı: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Model(String),
    #[error("{0}")]
    Network(String),
    #[error("Yanıt durduruldu. Konuşma geçmişi korundu.")]
    Cancelled,
    #[error("Yanıt otomatik devam sınırına ulaştı. Yazılan bölüm kaydedildi; Devam et ile sürdürebilirsiniz.")]
    OutputLimit,
}
