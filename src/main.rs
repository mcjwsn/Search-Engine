use std::error::Error;
use std::fs;
use crate::document::parser::parse_cisi_documents;

mod document;
mod preprocessing;
mod stemer;
mod matrix;
mod engine;

fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("data/cisi/cisi.all")?;

    let documents = parse_cisi_documents(&content);

    let stop_words = preprocessing::tokenizer::load_stop_words("stop_words/english.txt")?;

    let terms = preprocessing::tokenizer::build_vocabulary(&documents, &stop_words);

    let tfidf = matrix::TfIdfMatrix::build(&documents, &terms);

    let query = "information retrieval system";
    let results = engine::search::search(query, &tfidf, 5);

    for (doc_idx, score) in results {
        println!("Doc {} | Score: {:.4}", doc_idx, score);
        println!("Title: {}", documents[doc_idx].title);}

    Ok(())
}
