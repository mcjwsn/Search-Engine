use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::sync::Mutex;
use std::sync::Arc;

mod document;
mod preprocessing;
mod stemer;
mod matrix;
mod engine;

pub use document::parser::Document;

// Data structures for our API
#[derive(Serialize)]
struct SearchResult {
    id: String,
    score: f64,
    title: String,
    text: String,
    authors: Vec<String>,
}

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
    limit: Option<usize>,
}

// App state to hold our search index
struct AppState {
    documents: Arc<Vec<Document>>,
    tfidf_matrix: Arc<matrix::TfIdfMatrix>,
    terms: Arc<Vec<String>>,
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Information Retrieval API")
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
                id: doc.id.clone(),
                score,
                title: doc.title.clone(),
                text: doc.text.clone(),
                authors: doc.authors.clone(),
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Loading documents and building search index...");

    // Initialize search index
    let content = fs::read_to_string("data/cisi/cisi.all").expect("Failed to read documents file");
    let documents = document::parser::parse_cisi_documents(&content);

    let stop_words = preprocessing::tokenizer::load_stop_words("stop_words/english.txt")
        .expect("Failed to load stop words");

    let terms_map = preprocessing::tokenizer::build_vocabulary(&documents, &stop_words);
    let terms_vec: Vec<String> = terms_map.keys().cloned().collect();
    let tfidf = matrix::TfIdfMatrix::build(&documents, &terms_map);
    println!("Search index built successfully!");
    println!("Documents: {}", documents.len());
    println!("Vocabulary size: {}", terms_map.len());

    // Create app state
    let app_state = web::Data::new(Mutex::new(AppState {
        documents: Arc::new(documents),
        tfidf_matrix: Arc::new(tfidf),
        terms: Arc::new(terms_vec),
    }));

    println!("Starting HTTP server at http://127.0.0.1:8080");

    // Start HTTP server
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
            .service(stats)
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
