use std::collections::{HashMap, HashSet};
use sprs::{CsMat, TriMat};
use crate::document::parser::Document;

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

        // First pass: compute document frequencies (df) and collect term counts
        for (doc_index, doc) in documents.iter().enumerate() {
            let mut term_counts = HashMap::new();
            let mut seen_terms = HashSet::new();

            let text = format!("{} {}", doc.title, doc.text);
            let lowercased = text.to_lowercase();
            let tokens = lowercased
                .split_whitespace()
                .map(|t| t.trim_matches(|c: char| !c.is_alphabetic()));

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

            for (term_index, count) in term_counts {
                let tf = count as f64 / total_terms as f64;
                triplets.push((term_index, doc_index, tf));
            }
        }

        // Compute IDF
        let idf: Vec<f64> = df
            .iter()
            .map(|&df| {
                if df == 0 {
                    0.0
                } else {
                    (n_docs as f64 / df as f64).ln()
                }
            })
            .collect();

        // Compute TF-IDF and sort triplets before matrix construction
        let mut tf_idf_triplets: Vec<_> = triplets
            .into_iter()
            .map(|(term_index, doc_index, tf)| {
                let value = tf * idf[term_index];
                (term_index, doc_index, value)
            })
            .collect();

        // Sort triplets by column (doc_index) first, then by row (term_index)
        tf_idf_triplets.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| a.0.cmp(&b.0))
        });

        // Build the matrix
        let mut tri_mat = TriMat::new((n_terms, n_docs));
        for (row, col, val) in tf_idf_triplets {
            tri_mat.add_triplet(row, col, val);
        }

        // Convert to CSC format (requires sorted indices)
        let mut matrix = tri_mat.to_csc();

        // Normalize columns (document vectors)
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