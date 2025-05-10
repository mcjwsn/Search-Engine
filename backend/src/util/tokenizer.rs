use std::collections::{HashMap, HashSet};
use std::fs::File;
use nalgebra_sparse::CooMatrix;
use regex::Regex;
use crate::{util, Document};
use std::io::{BufRead, BufReader};

pub fn build_term_document_matrix(documents: &[Document]) -> (HashMap<String, usize>, HashMap<usize, String>, CooMatrix<f64>) {
    let stop_words = load_stop_words("english.txt").unwrap_or_else(|e| {
        eprintln!("Warning: Could not load stop words file: {}. Continuing without stop words.", e);
        HashSet::new()
    });

    let mut term_dict = HashMap::new();
    let mut inverse_term_dict = HashMap::new();
    let mut term_index = 0;

    for doc in documents {
        let tokens = tokenize(&doc.text);
        for token in tokens {
            if stop_words.contains(&token.to_lowercase()) {
                continue;
            }

            let stemmed_token = util::steming::porter_stem(&token);

            if !term_dict.contains_key(&stemmed_token) {
                term_dict.insert(stemmed_token.clone(), term_index);
                inverse_term_dict.insert(term_index, stemmed_token);
                term_index += 1;
            }
        }
    }

    println!("Dictionary built with {} terms (after stop words removal and stemming)", term_dict.len());

    let num_terms = term_dict.len();
    let num_docs = documents.len();

    let mut row_indices = Vec::new();
    let mut col_indices = Vec::new();
    let mut values = Vec::new();

    for (doc_idx, doc) in documents.iter().enumerate() {
        let tokens = tokenize(&doc.text);

        let mut term_counts = HashMap::new();
        for token in tokens {
            // Skip stop words
            if stop_words.contains(&token.to_lowercase()) {
                continue;
            }

            // Apply Porter stemming to the token before counting
            let stemmed_token = util::steming::porter_stem(&token);
            if let Some(&term_idx) = term_dict.get(&stemmed_token) {
                *term_counts.entry(term_idx).or_insert(0.0) += 1.0;
            }
        }

        for (term_idx, &count) in term_counts.iter() {
            row_indices.push(*term_idx);
            col_indices.push(doc_idx);
            values.push(count);
        }
    }

    let coo = CooMatrix::try_from_triplets(
        num_terms,
        num_docs,
        row_indices,
        col_indices,
        values,
    ).unwrap();

    (term_dict, inverse_term_dict, coo)
}

pub fn tokenize(text: &str) -> Vec<String> {
    let re = Regex::new(r"[^a-zA-Z0-9]+").unwrap();
    re.split(text)
        .filter(|s| !s.is_empty() && s.len() > 2)
        .map(|s| s.to_lowercase())
        .collect()
}

fn load_stop_words(filename: &str) -> std::io::Result<HashSet<String>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut stop_words = HashSet::new();

    for line in reader.lines() {
        let word = line?.trim().to_string();
        if !word.is_empty() {
            stop_words.insert(word);
        }
    }

    Ok(stop_words)
}