use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::Arc;
use crate::document::parser::parse_sqlite_documents;
use std::fs::File;
use sys_info;
use std::io::{BufReader, BufWriter};
// Ensure this import is correct based on your project structure
// use crate::engine::search::search_with_svd; // This was in the log, not the code snippet

use crate::matrix::SingularValueDecomposition; // This was in the problem description text
use rand; // Not used in the provided snippet, but was in the problem description text
use std::fs;
use std::path::Path;
mod document;
mod preprocessing;
mod stemer;
mod matrix;
mod engine;

pub use document::parser::Document;
const CACHED_DATA_PATH: &str = "search_index_cache.bin";
const SVD_CACHE_10: &str = "svd_cache_k10.bin";
const SVD_CACHE_25: &str = "svd_cache_k25.bin";
const SVD_CACHE_50: &str = "svd_cache_k50.bin";

#[derive(Serialize, Deserialize)]
struct CachedData {
    documents: Vec<Document>,
    terms: Vec<String>,
    tfidf_matrix: matrix::TfIdfMatrix,
}
#[derive(Serialize)]
struct SearchResult {
    id: i32,
    score: f64,
    title: String,
    text: String,
}

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
    limit: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchQueryWithSvd {
    query: String,
    limit: Option<usize>,
    use_svd: bool,
    k_value: Option<usize>,
}

struct AppState {
    documents: Arc<Vec<Document>>,
    tfidf_matrix: Arc<matrix::TfIdfMatrix>,
    terms: Arc<Vec<String>>,
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Information Retrieval API")
}

#[post("/search_svd")]
async fn search_svd(query: web::Json<SearchQueryWithSvd>, data: web::Data<Mutex<AppState>>) -> impl Responder {
    println!("Received SVD search request: {:?}", query);

    let state = data.lock().unwrap();
    let limit = query.limit.unwrap_or(10);
    let k_value = query.k_value.unwrap_or(10);

    let svd_path_str = match k_value {
        10 => SVD_CACHE_10,
        25 => SVD_CACHE_25,
        50 => SVD_CACHE_50,
        _ => {
            println!("Warning: Unsupported k_value {}, defaulting to k=10", k_value);
            SVD_CACHE_10
        },
    };

    println!("Attempting to load SVD from file: {}", svd_path_str);

    if !Path::new(svd_path_str).exists() {
        println!("SVD cache file {} does not exist. Listing current directory contents:", svd_path_str);
        match std::fs::read_dir(".") {
            Ok(entries) => {
                println!("Files in current directory:");
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Ok(metadata) = entry.metadata() {
                            println!("  {} - {} bytes", entry.path().display(), metadata.len());
                        } else {
                            println!("  {}", entry.path().display());
                        }
                    }
                }
            },
            Err(e) => println!("Error reading directory: {}", e),
        }
    }

    let results = match matrix::TfIdfMatrix::load_svd(svd_path_str) {
        Ok(svd) => {
            println!("Successfully loaded SVD from {}", svd_path_str);
            // It's good practice to ensure the loaded SVD corresponds to the tfidf_matrix dimensions
            // This might require passing matrix dimensions or the matrix itself to load_svd or checking after.
            // For now, we assume it's compatible if loaded.
            match std::panic::catch_unwind(|| {
                engine::search::search_with_svd(&query.query, &state.tfidf_matrix, &svd, limit)
            }) {
                Ok(search_results) => {
                    println!("SVD search completed successfully with k={}", k_value);
                    search_results
                },
                Err(e) => {
                    println!("Panic during SVD search (k={}): {:?}", k_value, e);
                    println!("Falling back to regular search");
                    engine::search::search(&query.query, &state.tfidf_matrix, limit)
                },
            }
        },
        Err(e) => {
            println!("Failed to load SVD from {} (k={}): {}", svd_path_str, k_value, e);
            println!("Falling back to regular search");
            engine::search::search(&query.query, &state.tfidf_matrix, limit)
        }
    };

    println!("Search returned {} results", results.len());

    let search_results: Vec<SearchResult> = results
        .into_iter()
        .map(|(doc_idx, score)| {
            let doc = &state.documents[doc_idx];
            SearchResult {
                id: doc.id.clone() as i32,
                score,
                title: doc.title.clone(),
                text: doc.text.clone(),
            }
        })
        .collect();

    HttpResponse::Ok().json(search_results)
}

#[post("/search")]
async fn search(query: web::Json<SearchQuery>, data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let limit = query.limit.unwrap_or(10);
    let results = engine::search::search(&query.query, &state.tfidf_matrix, limit);
    let search_results: Vec<SearchResult> = results
        .into_iter()
        .map(|(doc_idx, score)| {
            let doc = &state.documents[doc_idx];
            SearchResult {
                id: doc.id.clone() as i32,
                score,
                title: doc.title.clone(),
                text: doc.text.clone(),
            }
        })
        .collect();
    HttpResponse::Ok().json(search_results)
}

#[get("/stats")]
async fn stats(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = data.lock().unwrap();
    let stats = serde_json::json!({
        "document_count": state.documents.len(),
        "vocabulary_size": state.terms.len(),
    });
    HttpResponse::Ok().json(stats)
}

fn load_cached_data(path: &str) -> Result<CachedData, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let cached_data: CachedData = bincode::deserialize_from(reader)?;
    Ok(cached_data)
}

fn save_cached_data(data: &CachedData, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, data)?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    println!("Próba wczytania indeksu wyszukiwania z pamięci podręcznej...");

    let (documents_arc, terms_arc, tfidf_matrix_arc, base_tfidf_matrix) =
        match load_cached_data(CACHED_DATA_PATH) {
            Ok(cached_data) => {
                println!("Pomyślnie wczytano indeks wyszukiwania z pamięci podręcznej!");

                let loaded_tfidf_matrix = cached_data.tfidf_matrix;
                (
                    Arc::new(cached_data.documents),
                    Arc::new(cached_data.terms),
                    Arc::new(loaded_tfidf_matrix.clone()), // Clone for the Arc
                    loaded_tfidf_matrix,                   // Original matrix for SVD ops
                )
            }
            Err(e) => {
                println!("Nie udało się wczytać z pamięci podręcznej (Powód: {}). Budowanie od nowa...", e);
                println!("Ładowanie dokumentów i budowanie indeksu wyszukiwania...");

                let documents = parse_sqlite_documents("data/articles.db")
                    .expect("Nie udało się odczytać z SQLite");

                let stop_words_path = "stop_words/english.txt";
                let stop_words = preprocessing::tokenizer::load_stop_words(stop_words_path)
                    .unwrap_or_else(|err| {
                        eprintln!(
                            "Ostrzeżenie: Nie udało się załadować stop-words z {}: {}. Kontynuacja bez stop-words.",
                            stop_words_path, err
                        );
                        std::collections::HashSet::new()
                    });

                let terms_map = preprocessing::tokenizer::build_vocabulary(&documents, &stop_words);
                let mut terms_vec: Vec<String> = terms_map.keys().cloned().collect();
                terms_vec.sort_unstable();

                let new_tfidf_matrix = matrix::TfIdfMatrix::build(&documents, &terms_map);

                println!("Indeks wyszukiwania zbudowany pomyślnie!");
                println!("Dokumenty: {}", documents.len());
                println!("Rozmiar słownika: {}", terms_vec.len());

                let data_to_cache = CachedData {
                    documents: documents.clone(),
                    terms: terms_vec.clone(),
                    tfidf_matrix: new_tfidf_matrix.clone(),
                };

                if let Err(save_err) = save_cached_data(&data_to_cache, CACHED_DATA_PATH) {
                    eprintln!("Błąd podczas zapisywania indeksu do pamięci podręcznej: {}", save_err);
                } else {
                    println!("Indeks wyszukiwania zapisany do pamięci podręcznej: {}", CACHED_DATA_PATH);
                }

                // Return the newly built matrix components
                (
                    Arc::new(documents),
                    Arc::new(terms_vec),
                    Arc::new(new_tfidf_matrix.clone()), // Clone for the Arc
                    new_tfidf_matrix,                   // Original matrix for SVD ops
                )
            }
        };

    // SVD Caching Logic: Always check and create SVD if missing, using base_tfidf_matrix
    println!("\nChecking and computing SVD caches if necessary...");
    let svd_configs = [
        (10, SVD_CACHE_10),
        (25, SVD_CACHE_25),
        (50, SVD_CACHE_50),
    ];

    for (k_val, svd_path_str) in svd_configs.iter() {
        let k = *k_val; // Dereference k_val
        let svd_path = Path::new(svd_path_str);

        if !svd_path.exists() {
            println!("SVD cache for k={} not found at {}. Computing and saving...", k, svd_path.display());
            match base_tfidf_matrix.save_svd(k, svd_path_str) {
                Ok(_) => {
                    println!("Successfully saved SVD with k={} to {}", k, svd_path_str);
                    if let Ok(metadata) = fs::metadata(svd_path) {
                        println!("SVD file {} created with size: {} bytes", svd_path.display(), metadata.len());
                    } else {
                        println!("SVD file {} created, but could not get metadata.", svd_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Error saving SVD with k={} to {}: {}", k, svd_path_str, e);
                    if let Ok(metadata) = fs::metadata(".") {
                        eprintln!("  Current directory permissions: {:?}", metadata.permissions());
                    }
                    if let Ok(current_dir) = std::env::current_dir() {
                        eprintln!("  Current directory: {}", current_dir.display());
                    }
                    println!("WARNING: SVD file {} for k={} might not have been created due to error.", svd_path.display(), k);
                }
            }
        } else {
            if let Ok(metadata) = fs::metadata(svd_path) {
                println!("SVD cache for k={} found at {} ({} bytes). Skipping computation.", k, svd_path.display(), metadata.len());
            } else {
                println!("SVD cache for k={} found at {}. Skipping computation.", k, svd_path.display());
            }
        }
    }
    println!("SVD cache check complete.\n");


    let app_state = web::Data::new(Mutex::new(AppState {
        documents: documents_arc,
        tfidf_matrix: tfidf_matrix_arc,
        terms: terms_arc,
    }));

    println!("Uruchamianie serwera HTTP pod adresem http://127.0.0.1:8080");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .service(hello)
            .service(search)
            .service(search_svd)
            .service(stats)
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}