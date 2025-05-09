use std::collections::{HashMap, HashSet};
use sprs::{CsMat, TriMat};
use crate::document::parser::Document;
use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TfIdfMatrix {
    pub terms: HashMap<String, usize>,
    pub matrix: CsMat<f64>,
    pub idf: Vec<f64>,
}

impl TfIdfMatrix {
    pub fn build(documents: &[Document], terms: &HashMap<String, usize>) -> Self {
        let n_docs = documents.len();
        let n_terms = terms.len();

        let mut df = vec![0; n_terms];
        let mut triplets = Vec::new();

        for (doc_index, doc) in documents.iter().enumerate() {
            let mut term_counts = HashMap::new();
            let mut seen_terms = HashSet::new();

            let text = format!("{} {}", doc.title, doc.text);
            let lowercased = text.to_lowercase();
            // Simple tokenization: split by whitespace and remove non-alphabetic chars
            let tokens = lowercased
                .split_whitespace()
                .map(|t| t.trim_matches(|c: char| !c.is_alphabetic()))
                .filter(|t| !t.is_empty()); // Filter out empty strings after trimming


            let mut total_terms = 0;

            for token in tokens {
                if let Some(&term_index) = terms.get(token) {
                    *term_counts.entry(term_index).or_insert(0) += 1;
                    if seen_terms.insert(term_index) {
                        df[term_index] += 1;
                    }
                    total_terms += 1;
                }
            }

            // Avoid division by zero if a document is empty after filtering
            if total_terms > 0 {
                for (term_index, count) in term_counts {
                    let tf = count as f64 / total_terms as f64;
                    triplets.push((term_index, doc_index, tf));
                }
            }
        }

        let idf: Vec<f64> = df
            .iter()
            .map(|&df| {
                if df == 0 {
                    // If a term never appears, its IDF is effectively undefined or 0
                    // A common approach is log(N/0) -> infinity, but log(N/0) is inf.
                    // log( (N+1) / (df+1) ) is a common smoothing method.
                    // Or simply 0 if the term doesn't contribute. Let's use 0.0 for simplicity
                    // based on the original code's intent.
                    0.0
                } else {
                    // Apply log (natural logarithm)
                    (n_docs as f64 / df as f64).ln()
                }
            })
            .collect();

        let tf_idf_triplets: Vec<_> = triplets
            .into_iter()
            .map(|(term_index, doc_index, tf)| {
                let value = tf * idf[term_index];
                (term_index, doc_index, value)
            })
            .collect();

        // *** Potential Cause of Panic and Fix ***
        // sprs::TriMat::to_csc() expects the triplets, when grouped by column,
        // to have their row indices sorted. The way triplets are collected
        // above (iterating documents, then terms in hashmap order) does NOT
        // guarantee this. Sorting the triplets before converting to CSC ensures
        // that sprs receives data in the expected order.
        // Sort by column index first, then by row index for CSC format.
        let mut sorted_tf_idf_triplets = tf_idf_triplets;
        sorted_tf_idf_triplets.sort_by(|a, b| {
            a.1.cmp(&b.1) // Compare by column index (doc_index)
                .then(a.0.cmp(&b.0)) // Then compare by row index (term_index)
        });


        let mut tri_mat = TriMat::new((n_terms, n_docs));
        // Add sorted triplets to TriMat. Although TriMat can handle unsorted,
        // feeding it sorted data, especially before conversion, can sometimes
        // help internal processes or avoid edge cases in the conversion itself.
        // However, the primary fix is ensuring the *input* to the conversion
        // process is sortable in a way sprs expects. Sorting the raw triplets
        // seems the most direct way to influence this.
        for (row, col, val) in sorted_tf_idf_triplets {
            // sprs TriMat::add_triplet handles summing values for duplicate (row, col) entries
            tri_mat.add_triplet(row, col, val);
        }

        // Converting TriMat to CsMat (CSC format). This is where sprs
        // processes the accumulated triplets. Sorting them beforehand
        // should resolve the "Unsorted indices" panic.
        let mut matrix = tri_mat.to_csc();

        // L2 Normalization of columns (documents)
        for mut col in matrix.outer_iterator_mut() {
            let norm = col.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
            if norm > 0.0 {
                for (_, value) in col.iter_mut() {
                    *value /= norm;
                }
            }
        }

        Self {
            terms: terms.clone(),
            matrix,
            idf,
        }
    }
}