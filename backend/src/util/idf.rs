use nalgebra_sparse::{CooMatrix, CsrMatrix};

pub fn calculate_idf(term_doc_matrix: &CsrMatrix<f64>) -> Vec<f64> {
    let num_terms = term_doc_matrix.nrows();
    let num_docs = term_doc_matrix.ncols();
    let num_docs_f64 = num_docs as f64;

    let mut idf = vec![0.0; num_terms];

    for term_idx in 0..num_terms {
        let row_start = term_doc_matrix.row_offsets()[term_idx];
        let row_end = term_doc_matrix.row_offsets()[term_idx + 1];
        let mut doc_set = std::collections::HashSet::new();
        for idx in row_start..row_end {
            doc_set.insert(term_doc_matrix.col_indices()[idx]);
        }
        let doc_count = doc_set.len() as f64;

        if doc_count > 0.0 {
            idf[term_idx] = (num_docs_f64 / doc_count).ln();
        }
    }

    idf
}

pub fn apply_idf_weighting(term_doc_matrix: &mut CsrMatrix<f64>, idf: &[f64]) {
    let mut triplets = Vec::new();

    for i in 0..term_doc_matrix.nrows() {
        let row_start = term_doc_matrix.row_offsets()[i];
        let row_end = term_doc_matrix.row_offsets()[i + 1];

        for idx in row_start..row_end {
            let j = term_doc_matrix.col_indices()[idx];
            let val = term_doc_matrix.values()[idx];
            triplets.push((i, j, val * idf[i]));
        }
    }

    let coo = CooMatrix::try_from_triplets(
        term_doc_matrix.nrows(),
        term_doc_matrix.ncols(),
        triplets.iter().map(|(i, _, _)| *i).collect(),
        triplets.iter().map(|(_, j, _)| *j).collect(),
        triplets.iter().map(|(_, _, v)| *v).collect(),
    ).unwrap();

    *term_doc_matrix = CsrMatrix::from(&coo);
}