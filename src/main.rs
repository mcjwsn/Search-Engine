use std::error::Error;
use std::fs;

mod document;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let content = fs::read_to_string("data/cisi/cisi.all")?;

    let docs = document::parser::parse_cisi_documents(&content);

    for doc in docs {
        println!("ID: {}", doc.id);
        println!("Title: {}", doc.title);
        println!("Authors: {:?}", doc.authors);
        println!("Text: {}\n", doc.text);
    }

    Ok(())
}
