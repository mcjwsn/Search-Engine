use std::collections::{HashMap, HashSet};
use sprs::{CsMat, TriMat};
use crate::document::parser::Document;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use rand::Rng;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TfIdfMatrix {
    pub terms: HashMap<String, usize>,
    pub matrix: CsMat<f64>,
    pub idf: Vec<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SingularValueDecomposition {
    pub u: Vec<Vec<f64>>,     // Left singular vectors
    pub sigma: Vec<f64>,      // Singular values
    pub v_t: Vec<Vec<f64>>,   // Transposed right singular vectors
    pub k: usize,             // Rank of approximation
}

impl SingularValueDecomposition {
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }
}

impl TfIdfMatrix {

    pub fn compute_svd(&self, k: usize) -> SingularValueDecomposition {
        let n_terms = self.matrix.rows();
        let n_docs = self.matrix.cols();
        let mut u = vec![vec![0.0; k]; n_terms];
        let mut sigma = vec![0.0; k];
        let mut v_t = vec![vec![0.0; n_docs]; k];

        let mut remaining_matrix = self.matrix.clone();

        for i in 0..k {
            // Break if we've exceeded the rank
            if i >= std::cmp::min(n_terms, n_docs) {
                break;
            }

            let (s, u_i, v_i) = self.sparse_power_iteration(&remaining_matrix);

            sigma[i] = s;

            for j in 0..n_terms {
                u[j][i] = u_i[j];
            }

            for j in 0..n_docs {
                v_t[i][j] = v_i[j];
            }
            self.deflate_matrix(&mut remaining_matrix, s, &u_i, &v_i);
        }

        SingularValueDecomposition {
            u,
            sigma,
            v_t,
            k,
        }
    }

    fn power_iteration(&self, matrix: &Vec<Vec<f64>>) -> (f64, Vec<f64>, Vec<f64>) {
        let n_terms = matrix.len();
        let n_docs = matrix[0].len();

        // Start with a random vector
        let mut v = vec![0.0; n_docs];
        for i in 0..n_docs {
            v[i] = rand::random::<f64>() * 2.0 - 1.0;
        }

        // Normalize v
        let v_norm = (v.iter().map(|&x| x*x).sum::<f64>()).sqrt();
        for i in 0..n_docs {
            v[i] /= v_norm;
        }

        // Power iteration (simplified for demonstration)
        for _ in 0..30 {  // Fixed number of iterations for simplicity
            // u = A * v
            let mut u = vec![0.0; n_terms];
            for i in 0..n_terms {
                for j in 0..n_docs {
                    u[i] += matrix[i][j] * v[j];
                }
            }

            // Normalize u
            let u_norm = (u.iter().map(|&x| x*x).sum::<f64>()).sqrt();
            if u_norm < 1e-10 {
                break;  // Matrix is numerically zero in this subspace
            }

            for i in 0..n_terms {
                u[i] /= u_norm;
            }

            // v_new = A^T * u
            let mut v_new = vec![0.0; n_docs];
            for j in 0..n_docs {
                for i in 0..n_terms {
                    v_new[j] += matrix[i][j] * u[i];
                }
            }

            // Find sigma (singular value)
            let sigma = (v_new.iter().map(|&x| x*x).sum::<f64>()).sqrt();

            // Normalize v_new
            for j in 0..n_docs {
                v_new[j] /= sigma;
            }

            // Check for convergence
            let diff = v.iter().zip(v_new.iter())
                .map(|(&a, &b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();

            v = v_new;

            if diff < 1e-6 {
                break;
            }
        }

        // Calculate final u
        let mut u = vec![0.0; n_terms];
        for i in 0..n_terms {
            for j in 0..n_docs {
                u[i] += matrix[i][j] * v[j];
            }
        }

        // Calculate sigma
        let sigma = (u.iter().map(|&x| x*x).sum::<f64>()).sqrt();

        // Normalize u
        for i in 0..n_terms {
            u[i] /= sigma;
        }

        (sigma, u, v)
    }

    // Convert sparse matrix to dense for SVD calculations
    fn deflate_matrix(&self, matrix: &mut CsMat<f64>, sigma: f64, u: &[f64], v: &[f64]) {
        // This is a tricky operation to do efficiently on sparse matrices
        // We'll need to iterate through all non-zero elements and subtract the rank-1 update

        // Convert to triplet format for easier modification
        let mut tri_mat = TriMat::new((matrix.rows(), matrix.cols()));

        // Iterate through existing non-zero elements
        for col_idx in 0..matrix.cols() {
            if let Some(col_view) = matrix.outer_view(col_idx) {
                for (row_idx, &value) in col_view.iter() {
                    let new_value = value - sigma * u[row_idx] * v[col_idx];
                    // Only keep values above a certain threshold
                    if new_value.abs() > 1e-10 {
                        tri_mat.add_triplet(row_idx, col_idx, new_value);
                    }
                }
            }
        }

        // Convert back to CSC format
        *matrix = tri_mat.to_csc();
    }


    // Save SVD to file
    pub fn save_svd(&self, k: usize, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting SVD computation with k = {}...", k);

        // Verify dimensions before SVD computation
        let n_terms = self.matrix.rows();
        let n_docs = self.matrix.cols();
        println!("Matrix dimensions: {} terms x {} documents", n_terms, n_docs);

        // Check if matrix is empty
        if n_terms == 0 || n_docs == 0 {
            return Err("Matrix is empty, cannot compute SVD".into());
        }

        // Compute SVD with detailed timing
        let start_time = std::time::Instant::now();
        println!("Computing SVD...");

        let svd = self.compute_svd(k);

        let svd_time = start_time.elapsed();
        println!("SVD computation completed in {:.2?}", svd_time);

        // Verify SVD results
        println!("SVD dimensions: U: {}x{}, sigma: {}, V^T: {}x{}",
                 svd.u.len(), svd.u.get(0).map_or(0, |v| v.len()),
                 svd.sigma.len(),
                 svd.v_t.len(), svd.v_t.get(0).map_or(0, |v| v.len()));

        // Print a few singular values
        println!("First few singular values:");
        for (i, &sigma) in svd.sigma.iter().take(std::cmp::min(5, svd.sigma.len())).enumerate() {
            println!("  σ_{}: {:.6}", i, sigma);
        }

        // Save to file
        println!("Saving SVD to file: {}", path);
        let file_result = File::create(path);
        if let Err(ref e) = file_result {
            println!("Error creating file: {}", e);
            return Err(Box::new(std::io::Error::new(e.kind(),
                                                    format!("Failed to create file {}: {}", path, e))));
        }

        let file = file_result?;
        let writer = BufWriter::new(file);

        // Use detailed serialization with error reporting
        match bincode::serialize_into(writer, &svd) {
            Ok(_) => {
                println!("SVD successfully serialized and saved to {}", path);
                // Verify file exists and has content
                if let Ok(metadata) = std::fs::metadata(path) {
                    println!("File size: {} bytes", metadata.len());
                }
                Ok(())
            },
            Err(e) => {
                println!("Error serializing SVD: {}", e);
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other,
                                                 format!("Failed to serialize SVD: {}", e))))
            }
        }
    }

    fn sparse_power_iteration(&self, matrix: &CsMat<f64>) -> (f64, Vec<f64>, Vec<f64>) {
        let n_terms = matrix.rows();
        let n_docs = matrix.cols();

        // Start with a random vector
        let mut v = vec![0.0; n_docs];
        for i in 0..n_docs {
            v[i] = rand::random::<f64>() * 2.0 - 1.0;
        }

        // Normalize v
        let v_norm = (v.iter().map(|&x| x*x).sum::<f64>()).sqrt();
        for i in 0..n_docs {
            v[i] /= v_norm;
        }

        // Power iteration (simplified for demonstration)
        for _ in 0..30 {  // Fixed number of iterations for simplicity
            // u = A * v (sparse matrix-vector multiplication)
            let mut u = vec![0.0; n_terms];
            for (col_idx, v_val) in v.iter().enumerate() {
                if let Some(col_view) = matrix.outer_view(col_idx) {
                    for (row_idx, mat_val) in col_view.iter() {
                        u[row_idx] += mat_val * v_val;
                    }
                }
            }

            // Normalize u
            let u_norm = (u.iter().map(|&x| x*x).sum::<f64>()).sqrt();
            if u_norm < 1e-10 {
                break;  // Matrix is numerically zero in this subspace
            }

            for i in 0..n_terms {
                u[i] /= u_norm;
            }

            // v_new = A^T * u (sparse matrix transpose-vector multiplication)
            let mut v_new = vec![0.0; n_docs];
            for col_idx in 0..n_docs {
                if let Some(col_view) = matrix.outer_view(col_idx) {
                    for (row_idx, mat_val) in col_view.iter() {
                        v_new[col_idx] += mat_val * u[row_idx];
                    }
                }
            }

            // Find sigma (singular value)
            let sigma = (v_new.iter().map(|&x| x*x).sum::<f64>()).sqrt();

            // Normalize v_new
            for j in 0..n_docs {
                v_new[j] /= sigma;
            }

            // Check for convergence
            let diff = v.iter().zip(v_new.iter())
                .map(|(&a, &b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();

            v = v_new;

            if diff < 1e-6 {
                break;
            }
        }

        // Calculate final u using sparse multiplication
        let mut u = vec![0.0; n_terms];
        for (col_idx, v_val) in v.iter().enumerate() {
            if let Some(col_view) = matrix.outer_view(col_idx) {
                for (row_idx, mat_val) in col_view.iter() {
                    u[row_idx] += mat_val * v_val;
                }
            }
        }

        // Calculate sigma
        let sigma = (u.iter().map(|&x| x*x).sum::<f64>()).sqrt();

        // Normalize u
        for i in 0..n_terms {
            u[i] /= sigma;
        }

        (sigma, u, v)
    }

    // Load SVD from file
    pub fn load_svd(path: &str) -> Result<SingularValueDecomposition, Box<dyn std::error::Error>> {
        println!("[DEBUG] Attempting to load SVD from {}", path);
        let file = File::open(path)?;
        println!("[DEBUG] File opened successfully, size: {} bytes", file.metadata()?.len());

        let reader = BufReader::new(file);
        let svd: SingularValueDecomposition = bincode::deserialize_from(reader)
            .map_err(|e| {
                println!("[ERROR] Deserialization failed: {}", e);
                e
            })?;

        println!("[DEBUG] SVD loaded successfully. Dimensions: U={}x{}, Σ={}, Vᵗ={}x{}",
                 svd.u.len(), svd.u.get(0).map_or(0, |v| v.len()),
                 svd.sigma.len(),
                 svd.v_t.len(), svd.v_t.get(0).map_or(0, |v| v.len()));

        Ok(svd)
    }

    // Compute low-rank approximation of the TF-IDF matrix
    pub fn low_rank_approximation(&self, svd: &SingularValueDecomposition) -> CsMat<f64> {
        let k = svd.k;
        let n_terms = self.matrix.rows();
        let n_docs = self.matrix.cols();

        // Create a triplet matrix to build our approximation
        let mut tri_mat = TriMat::new((n_terms, n_docs));

        // Compute A_k = sum(sigma_i * u_i * v_i^T)
        for i in 0..k {
            if i >= svd.sigma.len() {
                break;
            }

            let sigma_i = svd.sigma[i];

            for row in 0..n_terms {
                let u_val = svd.u[row][i];

                for col in 0..n_docs {
                    let v_val = svd.v_t[i][col];
                    let value = sigma_i * u_val * v_val;

                    // Only add non-zero values to sparse matrix
                    if value.abs() > 1e-10 {
                        tri_mat.add_triplet(row, col, value);
                    }
                }
            }
        }

        // Convert to CSC format
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

        matrix
    }
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

        let mut sorted_tf_idf_triplets = tf_idf_triplets;
        sorted_tf_idf_triplets.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then(a.0.cmp(&b.0))
        });


        let mut tri_mat = TriMat::new((n_terms, n_docs));

        for (row, col, val) in sorted_tf_idf_triplets {
            tri_mat.add_triplet(row, col, val);
        }

        let mut matrix = tri_mat.to_csc();

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