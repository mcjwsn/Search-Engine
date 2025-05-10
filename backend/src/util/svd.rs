use std::error::Error;
use std::time::Instant;
use nalgebra::{DMatrix, DVector};
use nalgebra_sparse::CsrMatrix;
use rand::Rng;
use crate::{serialize_matrix, SvdData};

pub fn sparse_svd<F1, F2>(
    matrix_op: F1,
    transpose_op: F2,
    nrows: usize,
    ncols: usize,
    k: usize,
    max_iter: usize,
    tolerance: f64,
) -> Result<(DMatrix<f64>, Vec<f64>, DMatrix<f64>), Box<dyn Error>>
where
    F1: Fn(&[f64], &mut [f64]),
    F2: Fn(&[f64], &mut [f64]),
{
    let work_on_at_a = ncols <= nrows;
    let working_dim = if work_on_at_a { ncols } else { nrows };

    // Adjust k if it's too large for the matrix dimensions
    let k = k.min(working_dim).min(1000);

    let mut m = (2 * k).min(working_dim).min(max_iter);

    println!("Starting SVD computation for {k} components (working dim: {working_dim}, Lanczos steps: {m})");

    let mut q = vec![DVector::zeros(working_dim); m + 1];
    let mut alpha = vec![0.0; m];
    let mut beta = vec![0.0; m + 1];

    let mut rng = rand::thread_rng();
    for i in 0..working_dim {
        q[0][i] = rng.r#gen::<f64>() - 0.5;
    }
    q[0].normalize_mut();

    for i in 0..m {
        println!("Lanczos iteration {}/{}", i+1, m);

        let mut v = if work_on_at_a {
            let mut temp = vec![0.0; nrows];
            matrix_op(q[i].as_slice(), &mut temp);

            let mut result = vec![0.0; ncols];
            transpose_op(&temp, &mut result);
            DVector::from_vec(result)
        } else {
            let mut temp = vec![0.0; ncols];
            transpose_op(q[i].as_slice(), &mut temp);

            let mut result = vec![0.0; nrows];
            matrix_op(&temp, &mut result);
            DVector::from_vec(result)
        };

        for j in 0..=i {
            let dot = v.dot(&q[j]);
            v.axpy(-dot, &q[j], 1.0);
        }

        alpha[i] = v.dot(&q[i]);
        v.axpy(-alpha[i], &q[i], 1.0);

        if i > 0 {
            v.axpy(-beta[i], &q[i-1], 1.0);
        }

        for _ in 0..2 {
            for j in 0..=i {
                let dot = v.dot(&q[j]);
                v.axpy(-dot, &q[j], 1.0); }

        }

        beta[i+1] = v.norm();

        if beta[i+1] < tolerance || beta[i+1].is_nan() || beta[i+1].is_infinite() {
            println!("Early termination at iteration {} (beta = {})", i, beta[i+1]);
            m = i + 1;
            break;
        }

        q[i+1] = v / beta[i+1];
    }
    let mut t = DMatrix::zeros(m, m);
    for i in 0..m {
        t[(i, i)] = alpha[i];
        if i > 0 {
            t[(i, i-1)] = beta[i];
            t[(i-1, i)] = beta[i];
        }
    }

    println!("Computing eigenvalues of {}x{} tridiagonal matrix...", m, m);
    let eig = t.symmetric_eigen();
    let (eigenvalues, eigenvectors) = (eig.eigenvalues, eig.eigenvectors);

    let mut indices: Vec<usize> = (0..m).collect();
    indices.sort_by(|&a, &b| eigenvalues[b].partial_cmp(&eigenvalues[a]).unwrap());

    let sigma: Vec<f64> = indices.iter()
        .take(k)
        .map(|&i| {
            let lambda = eigenvalues[i];
            if lambda < -tolerance {
                println!("Warning: Found negative eigenvalue: {}", lambda);
                0.0
            } else {
                lambda.max(0.0).sqrt()
            }
        })
        .filter(|&s| s > tolerance)
        .collect();

    let actual_k = sigma.len();
    if actual_k < k {
        println!("Warning: Only found {actual_k} non-zero singular values (requested {k})");
    }
    if actual_k == 0 {
        return Err("No significant singular values found. Try reducing the tolerance.".into());
    }

    println!("Computing singular vectors...");
    let mut u = DMatrix::zeros(nrows, actual_k);
    let mut vt = DMatrix::zeros(actual_k, ncols);

    for (col, &idx) in indices.iter().take(actual_k).enumerate() {
        let theta = eigenvectors.column(idx);

        if work_on_at_a {
            let mut v_col = DVector::zeros(ncols);
            for j in 0..ncols {
                v_col[j] = (0..m).map(|l| q[l][j] * theta[l]).sum();
            }
            let mut u_col = DVector::zeros(nrows);
            matrix_op(v_col.as_slice(), u_col.as_mut_slice());

            if sigma[col] > tolerance * 10.0 {
                u_col /= sigma[col];
            } else {
                u_col.fill(0.0);
                println!("Warning: Small singular value {} at position {}", sigma[col], col);
            }

            u.set_column(col, &u_col);
            vt.set_row(col, &v_col.transpose());
        } else {
            let mut u_col = DVector::zeros(nrows);
            for j in 0..nrows {
                u_col[j] = (0..m).map(|l| q[l][j] * theta[l]).sum();
            }

            let mut v_col = DVector::zeros(ncols);
            transpose_op(u_col.as_slice(), v_col.as_mut_slice());

            if sigma[col] > tolerance * 10.0 {
                v_col /= sigma[col];
            } else {
                v_col.fill(0.0);
                println!("Warning: Small singular value {} at position {}", sigma[col], col);
            }

            u.set_column(col, &u_col);
            vt.set_row(col, &v_col.transpose());
        }
    }

    for i in 0..actual_k {
        let mut current_col = u.column(i).clone_owned();

        let mut dots = Vec::with_capacity(i);
        for j in 0..i {
            dots.push(current_col.dot(&u.column(j)));
        }

        for j in 0..i {
            let dot = dots[j];
            let col_j = u.column(j);
            for k in 0..current_col.len() {
                current_col[k] -= dot * col_j[k];
            }
        }

        let norm = current_col.norm().max(1e-10);
        current_col.scale_mut(1.0 / norm);
        u.column_mut(i).copy_from(&current_col);
    }

    for i in 0..actual_k {
        let mut current_row = vt.row(i).clone_owned();

        let mut dots = Vec::with_capacity(i);
        for j in 0..i {
            dots.push(current_row.dot(&vt.row(j)));
        }

        for j in 0..i {
            let dot = dots[j];
            let row_j = vt.row(j);
            for k in 0..current_row.len() {
                current_row[k] -= dot * row_j[k];
            }
        }

        let norm = current_row.norm().max(1e-10);
        current_row.scale_mut(1.0 / norm);
        vt.row_mut(i).copy_from(&current_row);
    }

    println!("SVD computation completed (effective rank: {actual_k})");
    Ok((u, sigma, vt))
}

pub fn perform_svd(term_doc_csr: &CsrMatrix<f64>, k: usize) -> Result<SvdData, Box<dyn Error>> {
    println!("Performing SVD with rank {}...", k);
    let start = Instant::now();
    let linear_op = |v: &[f64], result: &mut [f64]| {
        for i in 0..result.len() {
            result[i] = 0.0;
            let row_start = term_doc_csr.row_offsets()[i];
            let row_end = term_doc_csr.row_offsets()[i + 1];

            for idx in row_start..row_end {
                let j = term_doc_csr.col_indices()[idx];
                let val = term_doc_csr.values()[idx];
                result[i] += val * v[j];
            }
        }
    };

    let transpose_op = |v: &[f64], result: &mut [f64]| {
        for i in 0..result.len() {
            result[i] = 0.0;
        }

        for i in 0..term_doc_csr.nrows() {
            let row_start = term_doc_csr.row_offsets()[i];
            let row_end = term_doc_csr.row_offsets()[i + 1];

            for idx in row_start..row_end {
                let j = term_doc_csr.col_indices()[idx];
                let val = term_doc_csr.values()[idx];
                result[j] += val * v[i];
            }
        }
    };

    let (u, sigma, vt) = sparse_svd(
        linear_op,
        transpose_op,
        term_doc_csr.nrows(),
        term_doc_csr.ncols(),
        k,
        200, // 500
        1e-6,
    )?;

    println!("SVD computation completed in {:?}", start.elapsed());


    let actual_k = sigma.len();
    let mut doc_vectors = DMatrix::zeros(vt.ncols(), actual_k); // [n_docs x k]
    for j in 0..vt.ncols() {
        for i in 0..actual_k {
            doc_vectors[(j, i)] = sigma[i] * vt[(i, j)]; // vt[i,j] is V^T's element
        }
    }

    let svd_data = SvdData {
        rank: actual_k,
        sigma_k: sigma,
        u_ser: serialize_matrix(&u),
        vt_ser: serialize_matrix(&vt),
        docs_ser: serialize_matrix(&doc_vectors),
    };

    Ok(svd_data)
}