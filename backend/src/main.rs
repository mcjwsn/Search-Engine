use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::Arc;
use crate::document::parser::parse_sqlite_documents;
use std::fs::File;
use std::io::{BufReader, BufWriter};
mod document;
mod preprocessing;
mod stemer;
mod matrix;
mod engine;

pub use document::parser::Document;
const CACHED_DATA_PATH: &str = "search_index_cache.bin";
// Data structures for our API
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

// Funkcja pomocnicza do zapisywania danych w pamięci podręcznej
fn save_cached_data(data: &CachedData, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, data)?;
    Ok(())
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Próba wczytania indeksu wyszukiwania z pamięci podręcznej...");

    let (documents_arc, terms_arc, tfidf_matrix_arc) =
        match load_cached_data(CACHED_DATA_PATH) {
            Ok(cached_data) => {
                println!("Pomyślnie wczytano indeks wyszukiwania z pamięci podręcznej!");
                println!("Dokumenty: {}", cached_data.documents.len());
                println!("Rozmiar słownika: {}", cached_data.terms.len());
                (
                    Arc::new(cached_data.documents),
                    Arc::new(cached_data.terms),
                    Arc::new(cached_data.tfidf_matrix),
                )
            }
            Err(e) => {
                println!("Nie udało się wczytać z pamięci podręcznej (Powód: {}). Budowanie od nowa...", e);

                println!("Ładowanie dokumentów i budowanie indeksu wyszukiwania...");

                let documents = parse_sqlite_documents("data/articles.db") // Użyj document::parser::
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

                // Budowanie słownika (terms_map) jest potrzebne do zbudowania macierzy TF-IDF.
                // terms_vec (lista terminów) jest zapisywany.
                let terms_map = preprocessing::tokenizer::build_vocabulary(&documents, &stop_words);
                let mut terms_vec: Vec<String> = terms_map.keys().cloned().collect();
                terms_vec.sort_unstable(); // Opcjonalnie: sortowanie dla spójności, jeśli kolejność ma znaczenie

                // Budowanie macierzy TF-IDF
                // Uwaga: TfIdfMatrix::build może potrzebować terms_map, a nie terms_vec
                // Jeśli TfIdfMatrix::build używa indeksów z terms_map, upewnij się, że terms_vec
                // odpowiada tym indeksom (np. przez posortowanie terms_vec i użycie go do stworzenia mapy mapującej termin na nowy indeks)
                // Dla uproszczenia, zakładam, że build akceptuje terms_map.
                let tfidf_matrix = matrix::TfIdfMatrix::build(&documents, &terms_map);

                println!("Indeks wyszukiwania zbudowany pomyślnie!");
                println!("Dokumenty: {}", documents.len());
                println!("Rozmiar słownika: {}", terms_vec.len());

                // Przygotowanie danych do zapisu
                let data_to_cache = CachedData {
                    documents: documents.clone(), // Klonowanie jest potrzebne, bo `documents` jest potem przenoszone do Arc
                    terms: terms_vec.clone(),     // Podobnie dla `terms_vec`
                    tfidf_matrix: tfidf_matrix.clone(), // Podobnie dla `tfidf_matrix`
                };

                // Zapis danych do pamięci podręcznej
                if let Err(save_err) = save_cached_data(&data_to_cache, CACHED_DATA_PATH) {
                    eprintln!("Błąd podczas zapisywania indeksu do pamięci podręcznej: {}", save_err);
                } else {
                    println!("Indeks wyszukiwania zapisany do pamięci podręcznej: {}", CACHED_DATA_PATH);
                }

                (
                    Arc::new(documents),
                    Arc::new(terms_vec),
                    Arc::new(tfidf_matrix),
                )
            }
        };

    // Tworzenie stanu aplikacji
    let app_state = web::Data::new(Mutex::new(AppState {
        documents: documents_arc,
        tfidf_matrix: tfidf_matrix_arc,
        terms: terms_arc,
    }));

    println!("Uruchamianie serwera HTTP pod adresem http://127.0.0.1:8080");

    // Uruchamianie serwera HTTP
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