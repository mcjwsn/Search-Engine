use std::collections::{HashMap, HashSet};
use sprs::{CsMat, TriMat};
use ndarray::{Array2, Array1, s};
use ndarray_linalg::SVD;
use crate::document::parser::Document;

#[derive(Clone)]
pub struct TfIdfMatrix {
    pub terms: HashMap<String, usize>,
    pub matrix: CsMat<f64>,
    pub idf: Vec<f64>,
    pub svd_u: Option<Array2<f64>>,
    pub svd_sigma: Option<Array1<f64>>,
    pub svd_vt: Option<Array2<f64>>,
    pub low_rank_matrix: Option<CsMat<f64>>,
    pub k: Option<usize>,
}

impl TfIdfMatrix {
    pub fn build(documents: &[Document], terms: &HashMap<String, usize>) -> Self {
        let n_docs = documents.len();
        let n_terms = terms.len();

        let mut df = vec![0; n_terms]; // Document frequency dla każdego termu
        let mut triplets = Vec::new(); // (term_index, doc_index, tf)

        for (doc_index, doc) in documents.iter().enumerate() {
            let mut term_counts = HashMap::new();
            let mut seen_terms = HashSet::new();

            let text = format!("{} {}", doc.title, doc.text);
            let lowercased = text.to_lowercase(); // teraz wartość żyje odpowiednio długo

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

        // Oblicz IDF
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

        // Oblicz TF-IDF i zbuduj rzadką macierz
        let tf_idf_triplets: Vec<_> = triplets
            .into_iter()
            .map(|(term_index, doc_index, tf)| {
                let value = tf * idf[term_index];
                (term_index, doc_index, value)
            })
            .collect();

        let mut tri_mat = TriMat::new((n_terms, n_docs));
        for (row, col, val) in tf_idf_triplets {
            tri_mat.add_triplet(row, col, val);
        }

        let mut matrix = tri_mat.to_csc(); // macierz kolumnowo-rzadka

        for mut col in matrix.outer_iterator_mut() {
            let norm = col.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
            if norm > 0.0 {
                // Iterujemy przez dane w kolumnie i normalizujemy je
                for (_, value) in col.iter_mut() {
                    *value /= norm;
                }
            }
        }
        Self {
            terms: terms.clone(),
            matrix,
            idf,
            svd_u: None,
            svd_sigma: None,
            svd_vt: None,
            low_rank_matrix: None,
            k: None,
        }
    }

    // Nowa metoda do wykonania SVD i low rank approximation
    pub fn compute_svd_low_rank(&mut self, k: usize) {
        // Konwersja z rzadkiej macierzy do gęstej macierzy ndarray
        let (n_rows, n_cols) = self.matrix.shape();
        let mut dense_matrix = Array2::zeros((n_rows, n_cols));

        for (col_idx, col) in self.matrix.outer_iterator().enumerate() {
            for (row_idx, &value) in col.iter() {
                dense_matrix[[row_idx, col_idx]] = value;
            }
        }

        // Wykonanie SVD
        let svd = dense_matrix.svd(true, true).expect("SVD failed");
        let u = svd.0;
        let sigma = svd.1;
        let vt = svd.2.unwrap();

        // Sprawdź, czy mamy wystarczającą liczbę wartości osobliwych
        let k_actual = k.min(sigma.len());

        // Zachowaj tylko k największych wartości osobliwych i odpowiadające im wektory
        let u = u.expect("Left singular vectors (U) missing");
        let uk = u.slice(s![.., ..k_actual]).to_owned();
        let sigmak = sigma.slice(s![..k_actual]).to_owned();
        let vtk = vt.slice(s![..k_actual, ..]).to_owned();

        // Oblicz macierz przybliżoną Ak = Uk * Dk * Vk^T
        let mut ak_dense = Array2::zeros((n_rows, n_cols));
        for i in 0..k_actual {
            let ui = uk.slice(s![.., i]).to_owned().into_shape_with_order((n_rows, 1)).unwrap();
            let vi = vtk.slice(s![i, ..]).to_owned().into_shape_with_order((1, n_cols)).unwrap();
            let component = &ui * &vi * sigmak[i];
            ak_dense += &component;
        }

        // Konwersja z powrotem do macierzy rzadkiej
        let mut tri_mat = TriMat::<f64>::with_capacity((n_rows, n_cols), n_rows * n_cols); // or use TriMatI if needed

        for i in 0..n_rows {
            for j in 0..n_cols {
                let val = ak_dense[[i, j]];
                if val.abs() > 1e-10 {  // Ignorujemy bardzo małe wartości
                    tri_mat.add_triplet(i, j, val);
                }
            }
        }

        let low_rank_matrix = tri_mat.to_csc::<usize>();

        // Normalizacja kolumn low_rank_matrix
        let mut normalized_tri_mat = TriMat::new((n_rows, n_cols));
        for (col_idx, col) in low_rank_matrix.outer_iterator().enumerate() {
            let norm = col.iter().map(|(_, v)| v * v).sum::<f64>().sqrt();
            if norm > 0.0 {
                for (row_idx, &value) in col.iter() {
                    normalized_tri_mat.add_triplet(row_idx, col_idx, value / norm);
                }
            }
        }

        // Zapisz wyniki
        self.svd_u = Some(u);
        self.svd_sigma = Some(sigma);
        self.svd_vt = Some(vt);
        self.low_rank_matrix = Some(normalized_tri_mat.to_csc());
        self.k = Some(k_actual);
    }

    // Nowa metoda obliczająca podobieństwo według wzoru (5)
    pub fn cosine_similarity_with_noise_reduction(&self, query_vector: &[f64], document_idx: usize) -> f64 {
        if let Some(low_rank_matrix) = &self.low_rank_matrix {
            if document_idx >= low_rank_matrix.cols() {
                return 0.0;
            }

            // Wektor dokumentu z macierzy z usuniętym szumem (Ak*ej)
            if let Some(doc_vector) = low_rank_matrix.outer_view(document_idx) {
                // Obliczenie mianownika: ||q|| * ||Ak*ej||
                let query_norm = query_vector.iter().map(|&x| x * x).sum::<f64>().sqrt();
                let doc_norm = doc_vector.iter().map(|(_, &x)| x * x).sum::<f64>().sqrt();

                if query_norm == 0.0 || doc_norm == 0.0 {
                    return 0.0;
                }

                // Obliczenie licznika: q^T * Ak * ej
                let mut dot_product = 0.0;
                for (idx, &value) in doc_vector.iter() {
                    if idx < query_vector.len() {
                        dot_product += query_vector[idx] * value;
                    }
                }

                // Cosinus kąta
                dot_product / (query_norm * doc_norm)
            } else {
                0.0
            }
        } else {
            // Jeśli nie wykonaliśmy SVD, zwróć 0.0 lub możemy rzucić błąd
            eprintln!("SVD not computed. Call compute_svd_low_rank first.");
            0.0
        }
    }

    // Pomocnicza metoda do konwersji zapytania na wektor
    pub fn query_to_vector(&self, query: &str) -> Vec<f64> {
        let mut query_vector = vec![0.0; self.terms.len()];
        let lowercased = query.to_lowercase();
        let tokens: Vec<_> = lowercased
            .split_whitespace()
            .map(|t| t.trim_matches(|c: char| !c.is_alphabetic()))
            .collect();

        let mut term_counts = HashMap::new();
        let total_terms = tokens.len() as f64;

        if total_terms == 0.0 {
            return query_vector;
        }

        for token in tokens {
            if let Some(&term_index) = self.terms.get(token) {
                *term_counts.entry(term_index).or_insert(0.0) += 1.0;
            }
        }

        for (term_index, count) in term_counts {
            let tf = count / total_terms;
            let idf = self.idf[term_index];
            query_vector[term_index] = tf * idf;
        }

        // Normalizacja wektora zapytania
        let norm = query_vector.iter().map(|&x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for val in &mut query_vector {
                *val /= norm;
            }
        }

        query_vector
    }

    // Metoda do wykonania wyszukiwania z redukcją szumu
    pub fn search_with_noise_reduction(&self, query: &str, top_n: usize) -> Vec<(usize, f64)> {
        if self.low_rank_matrix.is_none() {
            eprintln!("SVD not computed. Call compute_svd_low_rank first.");
            return Vec::new();
        }

        let query_vector = self.query_to_vector(query);
        let n_docs = self.matrix.cols();

        let mut scores = Vec::with_capacity(n_docs);
        for doc_idx in 0..n_docs {
            let similarity = self.cosine_similarity_with_noise_reduction(&query_vector, doc_idx);
            scores.push((doc_idx, similarity));
        }

        // Sortuj według podobieństwa malejąco
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Zwróć top_n wyników
        scores.truncate(top_n);
        scores
    }

    // Metoda porównująca wyniki wyszukiwania z redukcją szumu i bez
    pub fn compare_search_results(&self, query: &str, top_n: usize) -> (Vec<(usize, f64)>, Vec<(usize, f64)>) {
        // Standardowe wyszukiwanie (bez redukcji szumu)
        let query_vector = self.query_to_vector(query);
        let n_docs = self.matrix.cols();

        let mut standard_scores = Vec::with_capacity(n_docs);
        for doc_idx in 0..n_docs {
            let mut similarity = 0.0;
            if let Some(doc_vector) = self.matrix.outer_view(doc_idx) {
                let doc_norm = doc_vector.iter().map(|(_, &x)| x * x).sum::<f64>().sqrt();
                let query_norm = query_vector.iter().map(|&x| x * x).sum::<f64>().sqrt();

                if doc_norm > 0.0 && query_norm > 0.0 {
                    let mut dot_product = 0.0;
                    for (idx, &value) in doc_vector.iter() {
                        if idx < query_vector.len() {
                            dot_product += query_vector[idx] * value;
                        }
                    }
                    similarity = dot_product / (doc_norm * query_norm);
                }
            }
            standard_scores.push((doc_idx, similarity));
        }

        standard_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        standard_scores.truncate(top_n);

        // Wyszukiwanie z redukcją szumu
        let svd_scores = if self.low_rank_matrix.is_some() {
            self.search_with_noise_reduction(query, top_n)
        } else {
            Vec::new()
        };

        (standard_scores, svd_scores)
    }
}