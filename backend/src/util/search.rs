use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;
use nalgebra::DVector;
use nalgebra_sparse::CsrMatrix;
use crate::{deserialize_matrix, util, Document, SvdData};


pub fn search<'a>(
    query: &'a str,
    term_dict: &'a HashMap<String, usize>,
    idf: &'a [f64],
    term_doc_matrix: &'a CsrMatrix<f64>,
    documents: &'a [Document],
    top_k: usize,
) -> Result<Vec<(&'a Document, f64)>, Box<dyn Error>> {
    let query_vec = create_query_vector(query, term_dict, idf);

    let scores = calculate_similarity(&query_vec, term_doc_matrix);

    let top_results = scores.iter()
        .take(top_k)
        .map(|&(doc_idx, score)| (&documents[doc_idx], score))
        .collect();

    Ok(top_results)
}

pub fn create_query_vector(query: &str, term_dict: &HashMap<String, usize>, idf: &[f64]) -> DVector<f64> {
    let num_terms = term_dict.len();
    let mut query_vec = DVector::zeros(num_terms);

    let tokens = util::tokenizer::tokenize(query);

    for token in tokens {
        if let Some(&term_idx) = term_dict.get(&token) {
            query_vec[term_idx] += 1.0;
        }
    }

    for term_idx in 0..num_terms {
        query_vec[term_idx] *= idf[term_idx];
    }

    let norm = query_vec.norm();
    if norm > 0.0 {
        query_vec /= norm;
    }

    query_vec
}

fn calculate_similarity(query_vec: &DVector<f64>, term_doc_matrix: &CsrMatrix<f64>) -> Vec<(usize, f64)> {
    let num_docs = term_doc_matrix.ncols();
    let mut scores = vec![0.0; num_docs];

    for i in 0..query_vec.len() {
        if query_vec[i] != 0.0 {
            let row_start = term_doc_matrix.row_offsets()[i];
            let row_end = term_doc_matrix.row_offsets()[i + 1];

            for idx in row_start..row_end {
                let j = term_doc_matrix.col_indices()[idx];
                let val = term_doc_matrix.values()[idx];
                scores[j] += query_vec[i] * val;
            }
        }
    }

    let mut doc_scores = Vec::with_capacity(num_docs);
    for (doc_idx, &score) in scores.iter().enumerate() {
        doc_scores.push((doc_idx, score));
    }

    doc_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    doc_scores
}

pub(crate) fn search_with_low_rank<'a>(
    query: &'a str,
    term_dict: &'a HashMap<String, usize>,
    idf: &'a [f64],
    svd_data: &'a SvdData,
    documents: &'a [Document],
    noise_filter_k: Option<usize>,
    top_k: usize,
) -> Result<Vec<(&'a Document, f64)>, Box<dyn Error>> {
    let query_vec = util::search::create_query_vector(query, term_dict, idf);

    let scores = calculate_similarity_low_rank_optimized(&query_vec, svd_data, noise_filter_k, top_k);

    let top_results = scores.iter()
        .map(|&(doc_idx, score)| (&documents[doc_idx], score))
        .collect();

    Ok(top_results)
}

fn calculate_similarity_low_rank_optimized(
    query_vec: &DVector<f64>,
    svd_data: &SvdData,
    reduced_k: Option<usize>,
    top_k: usize
) -> Vec<(usize, f64)> {
    println!("Calculating similarity using optimized low-rank approximation...");
    let start = Instant::now();

    let orig_k = svd_data.rank;

    let effective_k = match reduced_k {
        Some(k) => k.min(orig_k),
        None => orig_k
    };

    let u_k = deserialize_matrix(&svd_data.u_ser).columns(0, effective_k).into_owned();

    let doc_vecs = deserialize_matrix(&svd_data.docs_ser).rows(0, effective_k).into_owned();
    let num_docs = doc_vecs.ncols();

    let query_lsi = u_k.transpose() * query_vec;

    let query_norm = query_lsi.norm();
    let normalized_query = if query_norm > 1e-10 {
        &query_lsi / query_norm
    } else {
        println!("Warning: Query has near-zero norm in LSI space");
        return Vec::new();
    };

    let mut scores = Vec::with_capacity(num_docs);
    for j in 0..num_docs {
        let doc_vec = doc_vecs.column(j);
        let doc_norm = doc_vec.norm();

        let sim = if doc_norm > 1e-10 {
            normalized_query.dot(&(doc_vec / doc_norm))
        } else {
            0.0
        };

        scores.push((j, sim));
    }

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(top_k);

    println!("Optimized similarity calculation completed in {:?}", start.elapsed());
    scores
}

pub(crate) fn search_svd<'a>(
    query: &'a str,
    term_dict: &'a HashMap<String, usize>,
    idf: &'a [f64],
    svd_data: &'a SvdData,
    documents: &'a [Document],
    top_k: usize,
) -> Result<Vec<(&'a Document, f64)>, Box<dyn Error>> {
    let query_vec = util::search::create_query_vector(query, term_dict, idf);
    let scores = calculate_similarity_svd(&query_vec, svd_data);

    let top_results = scores.into_iter()
        .take(top_k)
        .map(|(doc_idx, score)| (&documents[doc_idx], score))
        .collect();

    Ok(top_results)
}

fn calculate_similarity_svd(
    query_vec: &DVector<f64>,
    svd_data: &SvdData
) -> Vec<(usize, f64)> {
    let u_k = svd_data.u_k();
    let doc_vecs = svd_data.doc_vectors();
    let num_docs = doc_vecs.ncols();

    let query_lsi = u_k.transpose() * query_vec;
    let query_norm = query_lsi.norm();

    let mut scores = Vec::with_capacity(num_docs);
    for j in 0..num_docs {
        let doc_vec = doc_vecs.column(j);
        let doc_norm = doc_vec.norm();

        let sim = if doc_norm > 1e-12 && query_norm > 1e-12 {
            query_lsi.dot(&doc_vec) / (query_norm * doc_norm)
        } else {
            0.0
        };
        scores.push((j, sim));
    }

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scores
}


