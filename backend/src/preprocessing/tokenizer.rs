use std::collections::{HashSet, HashMap};
use std::fs;
use std::error::Error;
use regex::Regex;
use crate::document::parser::Document;
use crate::stemer;

pub fn load_stop_words(path: &str) -> Result<HashSet<String>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let stop_words = content
        .lines()
        .map(|line| line.trim().to_lowercase())
        .filter(|line| !line.is_empty())
        .collect();
    Ok(stop_words)
}

pub fn build_vocabulary(documents: &Vec<Document>, stop_words: &HashSet<String>) -> HashMap<String, usize> {
    let re = Regex::new(r"[a-zA-Z]+").unwrap();
    let mut terms = HashSet::new();

    for doc in documents {
        let full_text = format!("{} {}", doc.title, doc.text);
        for word in re.find_iter(&full_text.to_lowercase()) {
            let term = word.as_str().to_string();
            if !stop_words.contains(&term) {
                let stemmed = stemer::porter_algorithm::porter_stem(&term);
                terms.insert(stemmed);// Porównujemy ze słowami w HashSet<String>
            }
        }
    }

    // Posortuj i stwórz słownik (słowo -> indeks)
    let mut term_list: Vec<String> = terms.into_iter().collect();
    term_list.sort();
    term_list
        .into_iter()
        .enumerate()
        .map(|(i, term)| (term, i))
        .collect()
}