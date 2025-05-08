use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
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
//->std::io::Result<()>
#[actix_web::main]
async fn main()   {
    println!("Loading documents and building search index...");

    // Initialize search index
    let content = fs::read_to_string("data/cisi/cisi.all").expect("Failed to read documents file");
    let documents = document::parser::parse_cisi_documents(&content);

    let stop_words = preprocessing::tokenizer::load_stop_words("stop_words/english.txt")
        .expect("Failed to load stop words");



    let terms_map = preprocessing::tokenizer::build_vocabulary(&documents, &stop_words);
    let terms_vec: Vec<String> = terms_map.keys().cloned().collect();
    let mut tfidf = matrix::TfIdfMatrix::build(&documents, &terms_map);
    println!("Search index built successfully!");
    println!("Documents: {}", documents.len());
    println!("Vocabulary size: {}", terms_map.len());



    let query = "twoje zapytanie testowe";
    let top_n = 10;

    println!("Wyszukiwanie standardowe:");
    let (standard_results, _) = tfidf.compare_search_results(query, top_n);
    for (i, (doc_idx, score)) in standard_results.iter().enumerate() {
        println!("{}. Dokument {}: {:.4}", i+1, doc_idx, score);
    }

    // Testowanie różnych wartości k
    let k_values = [5, 10, 20, 30, 50, 100];

    println!("\nTestowanie różnych wartości k dla redukcji szumu:");
    for &k in &k_values {
        tfidf.compute_svd_low_rank(k);
        let (_, svd_results) = tfidf.compare_search_results(query, top_n);

        println!("\nWyniki dla k={}:", k);
        for (i, (doc_idx, score)) in svd_results.iter().enumerate() {
            println!("{}. Dokument {}: {:.4}", i+1, doc_idx, score);
        }

        // Analiza podobieństwa wyników
        let mut common_docs = 0;
        for (doc_idx, _) in &standard_results {
            if svd_results.iter().any(|(idx, _)| idx == doc_idx) {
                common_docs += 1;
            }
        }
        println!("Wspólne dokumenty: {} z {} ({}%)",
                 common_docs, top_n, (common_docs as f64 / top_n as f64 * 100.0).round());
    }

    // Badanie wpływu IDF
    println!("\nBadanie wpływu przekształcenia IDF:");

    // Tworzenie kopii macierzy bez IDF (tylko TF)
    let mut tf_only = tfidf.clone();
    // Zastąp wszystkie współczynniki IDF przez 1.0
    for i in 0..tf_only.idf.len() {
        tf_only.idf[i] = 1.0;
    }

    // Porównaj wyniki wyszukiwania
    let (tfidf_results, _) = tfidf.compare_search_results(query, top_n);
    let (tf_results, _) = tf_only.compare_search_results(query, top_n);

    println!("\nWyniki z TF-IDF:");
    for (i, (doc_idx, score)) in tfidf_results.iter().enumerate() {
        println!("{}. Dokument {}: {:.4}", i+1, doc_idx, score);
    }

    println!("\nWyniki tylko z TF (bez IDF):");
    for (i, (doc_idx, score)) in tf_results.iter().enumerate() {
        println!("{}. Dokument {}: {:.4}", i+1, doc_idx, score);
    }

    // Analiza podobieństwa wyników
    let mut common_docs = 0;
    for (doc_idx, _) in &tfidf_results {
        if tf_results.iter().any(|(idx, _)| idx == doc_idx) {
            common_docs += 1;
        }
    }
    println!("Wspólne dokumenty: {} z {} ({}%)",
             common_docs, top_n, (common_docs as f64 / top_n as f64 * 100.0).round());



    // // Create app state
    // let app_state = web::Data::new(Mutex::new(AppState {
    //     documents: Arc::new(documents),
    //     tfidf_matrix: Arc::new(tfidf),
    //     terms: Arc::new(terms_vec),
    // }));
    //
    // println!("Starting HTTP server at http://127.0.0.1:8080");
    //
    // // Start HTTP server
    // HttpServer::new(move || {
    //     let cors = Cors::default()
    //         .allow_any_origin()
    //         .allow_any_method()
    //         .allow_any_header();
    //
    //     App::new()
    //         .wrap(cors)
    //         .app_data(app_state.clone())
    //         .service(hello)
    //         .service(search)
    //         .service(stats)
    // })
    //     .bind("127.0.0.1:8080")?
    //     .run()
    //     .await
}
