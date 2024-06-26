use rusqlite::{Connection, Result, params};
use std::sync::Arc;
use tokio::sync::Mutex;


pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self>{
        let conn = Connection::open(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn))
        })
    }
    pub async fn init(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audio_text (
                id TEXT PRIMARY KEY,
                audio TEXT NOT NULL,
                RAW TEXT NOT NULL,
                EMBEDDING BLOB NOT NULL

            )",
            [],
        )?;
        Ok(())
    }
    pub async fn insert(&self, id:&str, audio:&str , raw: &str, embedding: Option<&[f32]>) -> Result<i64> {
        let conn = self.conn.lock().await;
        let embedding_blob = embedding.map(|e| e.iter().flat_map(|&f| f.to_le_bytes()).collect::<Vec<u8>>());
        conn.execute(
            "INSERT INTO audio_text (id, audio ,RAW, EMBEDDING) VALUES (?1, ?2 ,?3, ?4)",
            params![id, audio,raw, embedding_blob],
        )?;
        Ok(conn.last_insert_rowid())
    }
    pub async fn get(&self, audio: &str) -> Result<Vec<(String, Option<Vec<f32>>) >> { 
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT RAW, EMBEDDING FROM audio_text WHERE audio = ?")?;
        let rows = stmt.query_map([audio], |row|{
            let raw: String = row.get(0)?;
            let embedding_blob: Option<Vec<u8>> = row.get(1)?;
            let embedding = embedding_blob.map(|blob| {
                blob.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<f32>>()
            });
            Ok((raw, embedding))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
    pub async fn check_if_audio_exists(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM audio_text WHERE id = ?1")?;
        let count: i64 = stmt.query_row(params![id], |row| row.get(0))?;
        Ok(count > 0)
    }
}

