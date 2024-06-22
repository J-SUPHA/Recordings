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
                RAW TEXT NOT NULL,
                EMBEDDING BLOB NOT NULL

            )",
            [],
        )?;
        Ok(())
    }
    pub async fn insert(&self, id:&str, raw: &str, embedding: Option<&[f32]>) -> Result<i64> {
        let conn = self.conn.lock().await;
        let embedding_blob = embedding.map(|e| e.iter().flat_map(|&f| f.to_le_bytes()).collect::<Vec<u8>>());
        conn.execute(
            "INSERT INTO audio_text (id, RAW, EMBEDDING) VALUES (?1, ?2, ?3)",
            params![id, raw, embedding_blob],
        )?;
        Ok(conn.last_insert_rowid())
    }
    pub async fn get(&self) -> Result<Vec<(i64, String, Option<Vec<f32>>) >> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, RAW, EMBEDDING FROM audio_text")?;
        let rows = stmt.query_map([], |row|{
            let id: i64 = row.get(0)?;
            let raw: String = row.get(1)?;
            let embedding_blob: Option<Vec<u8>> = row.get(2)?;
            let embedding = embedding_blob.map(|blob| {
                blob.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect::<Vec<f32>>()
            });
            Ok((id, raw, embedding))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}

