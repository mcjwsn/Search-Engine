use std::error::Error;
use std::fs;
use crate::document::parser::parse_cisi_documents;

mod document;
mod preprocessing;

fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("data/cisi/cisi.all")?;

    let documents = parse_cisi_documents(&content);

    let stop_words = preprocessing::tokenizer::load_stop_words("stop_words/english.txt")?;

    let vocabulary = preprocessing::tokenizer::build_vocabulary(&documents, &stop_words);

    println!("Vocabulary size: {}", vocabulary.len());
    for (term, index) in vocabulary.iter().take(10) {
        println!("{} -> {}", term, index);
    }

    Ok(())
}
