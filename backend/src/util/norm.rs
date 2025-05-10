use nalgebra_sparse::{CooMatrix, CsrMatrix};

pub fn normalize_columns(term_doc_matrix: &mut CsrMatrix<f64>) {
    let num_docs = term_doc_matrix.ncols();
    let mut col_norms = vec![0.0; num_docs];

    for i in 0..term_doc_matrix.nrows() {
        let row_start = term_doc_matrix.row_offsets()[i];
        let row_end = term_doc_matrix.row_offsets()[i + 1];

        for idx in row_start..row_end {
            let j = term_doc_matrix.col_indices()[idx];
            let val = term_doc_matrix.values()[idx];
            col_norms[j] += val * val;
        }
    }

    for norm in col_norms.iter_mut() {
        *norm = norm.sqrt();
    }

    let mut triplets = Vec::new();

    for i in 0..term_doc_matrix.nrows() {
        let row_start = term_doc_matrix.row_offsets()[i];
        let row_end = term_doc_matrix.row_offsets()[i + 1];

        for idx in row_start..row_end {
            let j = term_doc_matrix.col_indices()[idx];
            let val = term_doc_matrix.values()[idx];

            if col_norms[j] > 0.0 {
                triplets.push((i, j, val / col_norms[j]));
            }
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