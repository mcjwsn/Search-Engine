use std::collections::HashMap;
use sprs::CsVec;
use crate::matrix::TfIdfMatrix;

pub fn search(query: &str, tfidf: &TfIdfMatrix, k: usize) -> Vec<(usize, f64)> {
    let mut query_tf: HashMap<usize, usize> = HashMap::new();
    let mut total_terms = 0;

    let binding = query
        .to_lowercase();
    let q_tokens = binding
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphabetic()));

    for token in q_tokens {
        if let Some(&idx) = tfidf.terms.get(token) {
            *query_tf.entry(idx).or_insert(0) += 1;
            total_terms += 1;
        }
    }

    // Zbuduj wektor q (TF-IDF)
    let mut indices = Vec::new();
    let mut data = Vec::new();
    for (term_idx, count) in query_tf {
        let tf = count as f64 / total_terms as f64;
        let idf = tfidf.idf[term_idx];
        indices.push(term_idx);
        data.push(tf * idf);
    }
    let query_vec = CsVec::new(tfidf.terms.len(), indices, data);

    // Oblicz kosinusową podobieństwo
    let mut similarities = Vec::new();
    for doc_idx in 0..tfidf.matrix.cols() {
        let doc_vec = tfidf.matrix.outer_view(doc_idx).unwrap();

        let dot = query_vec.dot(&doc_vec);
        let norm_query = query_vec.dot(&query_vec).sqrt();
        let norm_doc = doc_vec.dot(&doc_vec).sqrt();

        if norm_query > 0.0 && norm_doc > 0.0 {
            let sim = dot / (norm_query * norm_doc);
            similarities.push((doc_idx, sim));
        }
    }

    // Zwróć top-k wyników
    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    similarities.truncate(k);
    similarities
}
