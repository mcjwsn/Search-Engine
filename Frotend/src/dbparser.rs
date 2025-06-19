use regex::Regex;
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::Path;
use glob::glob;

#[derive(Debug)]
struct Document {
    id: i32,
    title: String,
    url: String,
    text: String,
}

fn parse_file(file_path: &Path) -> Vec<Document> {
    let mut docs = Vec::new();

    let content = fs::read_to_string(file_path).expect("Nie udało się odczytać pliku");

    let re = Regex::new(r#"<doc id="(\d+)" url="(https?://[^"]+)" title="([^"]+)">([^<]+)</doc>"#)
        .expect("Błąd składni regexu");

    for cap in re.captures_iter(&content) {
        let doc = Document {
            id: cap[1].parse().unwrap_or(0),
            url: cap[2].to_string(),
            title: cap[3].to_string(),
            text: cap[4].to_string(),
        };
        docs.push(doc);
    }

    docs
}

fn create_db_and_insert_data(docs: &[Document]) -> Result<()> {
    let conn = Connection::open("articles.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS articles (
            id INTEGER PRIMARY KEY,
            title TEXT,
            url TEXT,
            text TEXT
        )",
        [],
    )?;

    let tx = conn.transaction()?;

    {
        let mut stmt = tx.prepare(
            "INSERT INTO articles (id, title, url, text) VALUES (?, ?, ?, ?)",
        )?;

        for doc in docs {
            stmt.execute(params![doc.id, doc.title, doc.url, doc.text])?;
        }
    }

    tx.commit()?;
    Ok(())
}

fn main() {
    let folders = ["AA/*", "AC/*", "AB/*"];

    for pattern in folders.iter() {
        let paths = glob(pattern).expect("Nieprawidłowy wzorzec glob");
        let mut all_docs = Vec::new();

        for entry in paths {
            match entry {
                Ok(path) => {
                    println!("Przetwarzam plik: {:?}", path.display());
                    let docs = parse_file(&path);
                    all_docs.extend(docs);
                }
                Err(e) => println!("Błąd podczas wczytywania pliku: {}", e),
            }
        }

        if !all_docs.is_empty() {
            match create_db_and_insert_data(&all_docs) {
                Ok(_) => println!("Dane zostały zapisane w bazie danych."),
                Err(e) => println!("Błąd przy zapisie do bazy danych: {}", e),
            }
        } else {
            println!("Brak danych do zapisania.");
        }
    }
}
