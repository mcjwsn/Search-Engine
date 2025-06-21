<<<<<<< Updated upstream
mod util;
use actix_cors::Cors;
use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use std::sync::Arc;
use std::path::Path;
use std::error::Error;
use serde::{Serialize, Deserialize};
use nalgebra_sparse::CsrMatrix;
use nalgebra::DMatrix;
use actix_web::get;
=======
use std::sync::Arc;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use crate::document::parser::parse_sqlite_documents;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use crate::matrix::SingularValueDecomposition;
use std::path::Path;
use std::mem;
mod document;
mod preprocessing;
mod stemer;
mod matrix;
mod engine;
>>>>>>> Stashed changes

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Document {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
struct PreprocessedData {
    term_dict: std::collections::HashMap<String, usize>,
    inverse_term_dict: std::collections::HashMap<usize, String>,
    idf: Vec<f64>,
    documents: Vec<Document>,
<<<<<<< Updated upstream
    term_doc_csr: SerializableCsrMatrix,
=======
    terms: Vec<String>,
    tfidf_matrix: matrix::TfIdfMatrix,
}

#[derive(Serialize)]
struct SearchResult {
    id: i32,
    score: f64,
    title: String,
    text: String,
>>>>>>> Stashed changes
}

#[derive(Serialize, Deserialize)]
struct SerMatrix {
    nrows: usize,
    ncols: usize,
    data: Vec<f64>,
}

#[derive(Serialize, Deserialize)]
struct SvdData {
    rank: usize,
    sigma_k: Vec<f64>,
    u_ser: SerMatrix,
    vt_ser: SerMatrix,
    docs_ser: SerMatrix,
}

#[derive(Serialize, Deserialize)]
struct SerializableCsrMatrix {
    nrows: usize,
    ncols: usize,
    row_offsets: Vec<usize>,
    col_indices: Vec<usize>,
    values: Vec<f64>,
}

struct AppState {
    preprocessed_data: Arc<PreprocessedData>,
    svd_data: Arc<SvdData>,
    k: usize,
    noise_filter_k: usize,
}

<<<<<<< Updated upstream
#[derive(Serialize)]
struct SearchResult {
    score: f64,
    title: String,
    url: String,
    id: i64,
    text: String,
}

#[derive(Serialize)]
struct StatsResponse {
    document_count: usize,
    vocabulary_size: usize,
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    limit: Option<usize>,
    method: Option<u8>, // 2 = TF-IDF, 3 = SVD/LSI, 4 = Low-rank
}

impl SerializableCsrMatrix {
    fn from_csr(csr: &CsrMatrix<f64>) -> Self {
        SerializableCsrMatrix {
            nrows: csr.nrows(),
            ncols: csr.ncols(),
            row_offsets: csr.row_offsets().to_vec(),
            col_indices: csr.col_indices().to_vec(),
            values: csr.values().to_vec(),
=======
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Loading search index from cache...");

    let (documents_arc, terms_arc, tfidf_matrix_arc, base_tfidf_matrix) = match load_cached_data(CACHED_DATA_PATH) {
        Ok(cached_data) => {
            println!("Successfully loaded search index from cache!");
            let loaded_tfidf_matrix = cached_data.tfidf_matrix;
            (
                Arc::new(cached_data.documents),
                Arc::new(cached_data.terms),
                Arc::new(loaded_tfidf_matrix.clone()),
                loaded_tfidf_matrix,
            )
        }
        Err(e) => {
            println!("Failed to load from cache (Reason: {}). Rebuilding...", e);
            println!("Loading documents and building search index...");

            let documents = parse_sqlite_documents("data/articles.db")
                .expect("Failed to read from SQLite");

            let stop_words_path = "stop_words/english.txt";
            let stop_words = preprocessing::tokenizer::load_stop_words(stop_words_path)
                .unwrap_or_else(|err| {
                    eprintln!(
                        "Warning: Failed to load stop-words from {}: {}. Continuing without stop-words.",
                        stop_words_path, err
                    );
                    std::collections::HashSet::new()
                });

            let terms_map = preprocessing::tokenizer::build_vocabulary(&documents, &stop_words);
            let mut terms_vec: Vec<String> = terms_map.keys().cloned().collect();
            terms_vec.sort_unstable();

            let new_tfidf_matrix = matrix::TfIdfMatrix::build(&documents, &terms_map);

            println!("Search index built successfully!");
            println!("Documents: {}", documents.len());
            println!("Vocabulary size: {}", terms_vec.len());

            let data_to_cache = CachedData {
                documents: documents.clone(),
                terms: terms_vec.clone(),
                tfidf_matrix: new_tfidf_matrix.clone(),
            };

            if let Err(save_err) = save_cached_data(&data_to_cache, CACHED_DATA_PATH) {
                eprintln!("Error saving index to cache: {}", save_err);
            } else {
                println!("Search index saved to cache: {}", CACHED_DATA_PATH);
            }

            (
                Arc::new(documents),
                Arc::new(terms_vec),
                Arc::new(new_tfidf_matrix.clone()),
                new_tfidf_matrix,
            )
        }
    };

    // Sample queries for testing
    let sample_queries = vec![
        "science",
        "technology",
        "health",
        "artificial intelligence",
        "climate change",
    ];

    // Process each SVD configuration one at a time to manage memory
    let svd_configs = [
        (10, SVD_CACHE_10),
        (25, SVD_CACHE_25),
        (50, SVD_CACHE_50),
    ];

    for (k, path) in &svd_configs {
        println!("\n==================================================================");
        println!("PROCESSING SVD CONFIGURATION WITH k={}", k);
        println!("==================================================================");

        // Load or compute SVD
        let svd = if Path::new(path).exists() {
            println!("Loading SVD from cache: {}", path);
            match matrix::TfIdfMatrix::load_svd(path) {
                Ok(svd) => {
                    println!("Successfully loaded SVD with k={}", k);
                    svd
                }
                Err(e) => {
                    println!("Failed to load SVD from cache, computing fresh: {}", e);
                    let svd = base_tfidf_matrix.compute_svd(*k);
                    if let Err(e) = svd.save(path) {
                        eprintln!("Warning: Failed to save computed SVD: {}", e);
                    }
                    svd
                }
            }
        } else {
            println!("No cache found, computing fresh SVD with k={}", k);
            let svd = base_tfidf_matrix.compute_svd(*k);
            if let Err(e) = svd.save(path) {
                eprintln!("Warning: Failed to save computed SVD: {}", e);
            }
            svd
        };

        // Test each query with the current SVD configuration
        for query in &sample_queries {
            println!("\n--------------------------------------------------");
            println!("Testing query: '{}' with k={}", query, k);

            // SVD search
            let start = std::time::Instant::now();
            let svd_results = engine::search::search_with_svd(query, &base_tfidf_matrix, &svd, 5);
            let svd_time = start.elapsed();

            println!("SVD search results (k={}, took {:?}):", k, svd_time);
            for (i, (doc_idx, score)) in svd_results.iter().enumerate() {
                println!("{}. {} (score: {:.4})", i+1, documents_arc[*doc_idx].title, score);
            }

            // Force a small delay to let the system process memory
            std::thread::sleep(std::time::Duration::from_millis(100));
>>>>>>> Stashed changes
        }

        // Clean up and free memory before next iteration
        drop(svd);
        println!("\nFinished testing with k={}, memory freed", k);

        // Force garbage collection by suggesting a heap compact
        unsafe { libc::malloc_trim(0); }
    }

<<<<<<< Updated upstream
    fn to_csr(&self) -> CsrMatrix<f64> {
        CsrMatrix::try_from_csr_data(
            self.nrows,
            self.ncols,
            self.row_offsets.clone(),
            self.col_indices.clone(),
            self.values.clone(),
        ).unwrap()
    }
}

impl SvdData {
    fn u_k(&self) -> DMatrix<f64> {
        DMatrix::from_row_slice(
            self.u_ser.nrows,
            self.u_ser.ncols,
            &self.u_ser.data
        )
    }

    fn doc_vectors(&self) -> DMatrix<f64> {
        DMatrix::from_row_slice(
            self.docs_ser.nrows,
            self.docs_ser.ncols,
            &self.docs_ser.data
        )
    }

    pub fn effective_rank(&self, requested_k: Option<usize>) -> usize {
        requested_k.map(|k| k.min(self.rank)).unwrap_or(self.rank)
    }

    pub fn get_u_k(&self, requested_k: Option<usize>) -> DMatrix<f64> {
        let k = self.effective_rank(requested_k);
        self.u_k().columns(0, k).into_owned()
    }

    pub fn get_doc_vectors(&self, requested_k: Option<usize>) -> DMatrix<f64> {
        let k = self.effective_rank(requested_k);
        self.doc_vectors().rows(0, k).into_owned()
    }
}

#[get("/stats")]
async fn get_stats(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(StatsResponse {
        document_count: data.preprocessed_data.documents.len(),
        vocabulary_size: data.preprocessed_data.term_dict.len(),
    })
=======
    // Now run a web server to provide the search functionality
    println!("\nStarting web server at http://127.0.0.1:8080");

    let app_state = web::Data::new(AppState {
        documents: documents_arc.clone(),
        terms: terms_arc,
        tfidf_matrix: tfidf_matrix_arc,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(search)
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

#[actix_web::post("/search")]
pub async fn search(
    query: web::Json<SearchQueryWithSvd>,
    state: web::Data<AppState>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(10);
    let k_value = query.k_value.unwrap_or(25);

    if query.use_svd {
        // Load appropriate SVD model based on k_value
        let svd_path = match k_value {
            k if k <= 10 => SVD_CACHE_10,
            k if k <= 25 => SVD_CACHE_25,
            _ => SVD_CACHE_50,
        };

        // Load SVD from cache
        match matrix::TfIdfMatrix::load_svd(svd_path) {
            Ok(svd) => {
                let results = engine::search::search_with_svd(
                    &query.query,
                    &state.tfidf_matrix,
                    &svd,
                    limit
                );

                let search_results: Vec<SearchResult> = results
                    .into_iter()
                    .map(|(doc_idx, score)| {
                        let doc = &state.documents[doc_idx];
                        SearchResult {
                            id: doc.id as i32,
                            score,
                            title: doc.title.clone(),
                            text: doc.text.clone(),
                        }
                    })
                    .collect();

                HttpResponse::Ok().json(search_results)
            },
            Err(e) => {
                HttpResponse::InternalServerError().body(format!("Failed to load SVD: {}", e))
            }
        }
    } else {
        // Regular search without SVD
        let results = engine::search::search(&query.query, &state.tfidf_matrix, limit);

        let search_results: Vec<SearchResult> = results
            .into_iter()
            .map(|(doc_idx, score)| {
                let doc = &state.documents[doc_idx];
                SearchResult {
                    id: doc.id as i32,
                    score,
                    title: doc.title.clone(),
                    text: doc.text.clone(),
                }
            })
            .collect();

        HttpResponse::Ok().json(search_results)
    }
>>>>>>> Stashed changes
}

async fn search_handler(
    data: web::Data<AppState>,
    req: web::Json<SearchRequest>,
) -> impl Responder {
    let query = &req.query;
    let top_k = req.limit.unwrap_or(10);
    let method = req.method.unwrap_or(2); // Domyślnie TF-IDF

    let csr = data.preprocessed_data.term_doc_csr.to_csr();

    let results = match method {
        2 => {
            // Standard TF-IDF search
            util::search::search(
                query,
                &data.preprocessed_data.term_dict,
                &data.preprocessed_data.idf,
                &csr,
                &data.preprocessed_data.documents,
                top_k,
            )
        }
        3 => {
            // SVD/LSI search
            util::search::search_svd(
                query,
                &data.preprocessed_data.term_dict,
                &data.preprocessed_data.idf,
                &data.svd_data,
                &data.preprocessed_data.documents,
                top_k,
            )
        }
        4 => {
            // Low-rank approximation with noise filtering
            util::search::search_with_low_rank(
                query,
                &data.preprocessed_data.term_dict,
                &data.preprocessed_data.idf,
                &data.svd_data,
                &data.preprocessed_data.documents,
                Some(data.noise_filter_k),
                top_k,
            )
        }
        _ => {
            return HttpResponse::BadRequest().body("Invalid search method. Use 2 (TF-IDF), 3 (SVD/LSI), or 4 (Low-rank)");
        }
    };

    match results {
        Ok(results) => HttpResponse::Ok().json(
            results.into_iter()
                .map(|(doc, score)| SearchResult {
                    score,
                    title: doc.title.clone(),
                    url: doc.url.clone(),
                    id: doc.id,
                    text: doc.text.clone(),
                })
                .collect::<Vec<_>>()
        ),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

<<<<<<< Updated upstream
#[get("/document/{id}")]
async fn get_document(
    data: web::Data<AppState>,
    id: web::Path<i64>,
) -> impl Responder {
    let doc_id = id.into_inner();

    if let Some(doc) = data.preprocessed_data.documents.iter().find(|d| d.id == doc_id) {
        HttpResponse::Ok().json(SearchResult {
            score: 0.0,
            title: doc.title.clone(),
            url: doc.url.clone(),
            id: doc.id,
            text: doc.text.clone(),
        })
    } else {
        HttpResponse::NotFound().body("Document not found")
    }
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db_path = "../Search-Engine/backend/data/articles.db";
    let preproc_index = "preprocessed.idx";
    let svd_index = |k| format!("svd_k{}.idx", k);

    let pre = if Path::new(preproc_index).exists() {
        println!("Loading preprocessed data...");
        util::data::load_preprocessed_data(preproc_index)?
    } else {
        println!("Building index from SQLite...");
        let docs = util::parser::parse_sqlite_documents(db_path)?;
        let (term_dict, inv_term_dict, coo) = util::tokenizer::build_term_document_matrix(&docs);
        let mut csr = CsrMatrix::from(&coo);
        let idf = util::idf::calculate_idf(&csr);
        util::idf::apply_idf_weighting(&mut csr, &idf);
        util::norm::normalize_columns(&mut csr);

        let pre = PreprocessedData {
            term_dict,
            inverse_term_dict: inv_term_dict,
            idf,
            documents: docs,
            term_doc_csr: SerializableCsrMatrix::from_csr(&csr),
        };
        util::data::save_preprocessed_data(&pre, preproc_index)?;
        pre
    };

    let k = 25;
    println!("Using SVD rank k={}", k);

    let svd_data = if Path::new(&svd_index(k)).exists() {
        println!("Loading SVD data (k={})...", k);
        util::data::load_svd_data(&svd_index(k))?
    } else {
        println!("Performing SVD with k={}...", k);
        let csr = pre.term_doc_csr.to_csr();
        let svd = util::svd::perform_svd(&csr, k)?;
        util::data::save_svd_data(&svd, &svd_index(k))?;
        svd
    };

    let noise_filter_k = k;

    let state = web::Data::new(AppState {
        preprocessed_data: Arc::new(pre),
        svd_data: Arc::new(svd_data),
        k,
        noise_filter_k,
    });

    println!("Starting API server on http://127.0.0.1:8080");
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .service(get_stats)
            .service(get_document)
            .route("/search", web::post().to(search_handler))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await?;

    Ok(())
}

fn serialize_matrix(m: &DMatrix<f64>) -> SerMatrix {
    SerMatrix {
        nrows: m.nrows(),
        ncols: m.ncols(),
        data: m.iter().cloned().collect(),
    }
}
fn deserialize_matrix(s: &SerMatrix) -> DMatrix<f64> {
    DMatrix::from_row_slice(s.nrows, s.ncols, &s.data)
=======
fn save_cached_data(data: &CachedData, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, data)?;
    Ok(())
>>>>>>> Stashed changes
}