use rusqlite::{Connection, Result};
use std::path::Path;

#[derive(Debug)]
pub struct Document {
    pub id: i64,          // Changed to i64 for INTEGER primary key
    pub title: String,
    pub url: String,      // Added url field
    pub text: String,
    pub authors: Vec<String>,
}

pub fn parse_sqlite_documents(db_path: &str) -> Result<Vec<Document>, rusqlite::Error> {
    let conn = Connection::open(Path::new(db_path))?;

    let mut stmt = conn.prepare("SELECT id, title, url, text FROM articles")?;
    let document_iter = stmt.query_map([], |row| {
        Ok(Document {
            id: row.get(0)?,       // i64
            title: row.get(1)?,    // String
            url: row.get(2)?,      // String
            text: row.get(3)?,     // String
            authors: Vec::new(),   // Will be populated separately
        })
    })?;

    let mut documents = Vec::new();
    for doc in document_iter {
        documents.push(doc?);
    }

    Ok(documents)
}