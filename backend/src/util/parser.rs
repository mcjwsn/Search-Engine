use std::path::Path;
use crate::Document;
use rusqlite::{Connection, Result as SqliteResult};


pub fn parse_sqlite_documents(db_path: &str) -> SqliteResult<Vec<Document>> {
    let conn = Connection::open(Path::new(db_path))?;

    let mut stmt = conn.prepare("SELECT id, title, url, text FROM articles")?;
    let document_iter = stmt.query_map([], |row| {
        Ok(Document {
            id: row.get(0)?,
            title: row.get(1)?,
            url: row.get(2)?,
            text: row.get(3)?,
        })
    })?;

    let mut documents = Vec::new();
    for doc in document_iter {
        documents.push(doc?);
    }

    Ok(documents)
}