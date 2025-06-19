use reqwest::Client;
use scraper::{Html, Selector};
use tokio_rusqlite::Connection;
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, Mutex};
use futures::future::join_all;
use log::{info, warn, error};

const NUM_ARTICLES: usize = 300000;
const WIKIPEDIA_RANDOM_URL: &str = "https://pl.wikipedia.org/wiki/Special:Random";
const CONCURRENT_REQUESTS: usize = 10;
const DELAY_BETWEEN_BATCHES_MS: u64 = 100;
const PROGRESS_UPDATE_INTERVAL: usize = 100;

#[derive(Debug)]
struct Article {
    url: String,
    title: String,
    text: String,
}

#[derive(Debug, Clone)]
struct ProgressCounter {
    completed: Arc<Mutex<usize>>,
    saved: Arc<Mutex<usize>>,
    start_time: Instant,
}

impl ProgressCounter {
    fn new() -> Self {
        Self {
            completed: Arc::new(Mutex::new(0)),
            saved: Arc::new(Mutex::new(0)),
            start_time: Instant::now(),
        }
    }

    async fn increment_completed(&self) -> usize {
        let mut count = self.completed.lock().await;
        *count += 1;
        *count
    }

    async fn increment_saved(&self) -> usize {
        let mut count = self.saved.lock().await;
        *count += 1;
        *count
    }

    async fn print_progress(&self, completed: usize) {
        if completed % PROGRESS_UPDATE_INTERVAL == 0 || completed == NUM_ARTICLES {
            let saved = *self.saved.lock().await;
            let elapsed = self.start_time.elapsed();
            let rate = completed as f64 / elapsed.as_secs_f64();
            let eta_seconds = if rate > 0.0 {
                ((NUM_ARTICLES - completed) as f64 / rate) as u64
            } else {
                0
            };

            let eta_hours = eta_seconds / 3600;
            let eta_minutes = (eta_seconds % 3600) / 60;
            let eta_secs = eta_seconds % 60;

            println!(
                "Postęp: {}/{} ({:.1}%) | Zapisane: {} | Czas: {:.1}s | Szybkość: {:.1}/s | ETA: {:02}:{:02}:{:02}",
                completed,
                NUM_ARTICLES,
                (completed as f64 / NUM_ARTICLES as f64) * 100.0,
                saved,
                elapsed.as_secs_f64(),
                rate,
                eta_hours,
                eta_minutes,
                eta_secs
            );
        }
    }
}

async fn create_table_if_not_exists(conn: &Connection) -> Result<()> {
    conn.call(|db_conn| {
        db_conn.execute(
            "CREATE TABLE IF NOT EXISTS articles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT UNIQUE,
                title TEXT,
                text TEXT
            )",
            [],
        )?;
        Ok(())
    }).await?;
    info!("Tabela 'articles' sprawdzona/utworzona.");
    Ok(())
}

async fn scrape_random_article(client: &Client) -> Result<Option<Article>> {
    let response = client.get(WIKIPEDIA_RANDOM_URL)
        .send()
        .await?;

    let final_url = response.url().to_string();
    let html_content = response.text().await?;
    let document = Html::parse_document(&html_content);

    let title_selector = Selector::parse("h1#firstHeading").expect("Błędny selektor tytułu");
    let title = document
        .select(&title_selector)
        .next()
        .map(|element| element.text().collect::<String>())
        .unwrap_or_else(|| "Brak tytułu".to_string());

    let paragraph_selector = Selector::parse("div.mw-parser-output > p, div.mw-content-ltr > div.mw-parser-output > p").expect("Błędny selektor paragrafów");
    let mut paragraphs_text = Vec::new();
    for p_element in document.select(&paragraph_selector) {
        let text_content = p_element.text().collect::<String>();
        if !text_content.trim().is_empty() {
            paragraphs_text.push(text_content);
        }
    }
    let text = paragraphs_text.join("\n");

    if title == "Brak tytułu" || text.is_empty() {
        warn!("Nie udało się sparsować tytułu lub tekstu dla URL: {}", final_url);
        return Ok(None);
    }

    Ok(Some(Article {
        url: final_url,
        title,
        text,
    }))
}

async fn save_article(conn: &Connection, article: &Article) -> Result<bool> {
    let article_url = article.url.clone();
    let article_title = article.title.clone();
    let article_text = article.text.clone();

    let rows_affected = conn.call(move |db_conn| {
        db_conn.execute(
            "INSERT OR IGNORE INTO articles (url, title, text) VALUES (?, ?, ?)",
            &[&article_url, &article_title, &article_text],
        )
    }).await?;

    Ok(rows_affected > 0)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Rozpoczynanie scrapowania {} artykułów z Wikipedii...", NUM_ARTICLES);

    let db_conn = Connection::open("articles.db").await?;
    create_table_if_not_exists(&db_conn).await?;

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::default())
        .build()?;

    let semaphore = Arc::new(Semaphore::new(CONCURRENT_REQUESTS));
    let progress = ProgressCounter::new();
    let mut tasks = Vec::new();

    println!("Tworzenie {} równoczesnych zadań...", NUM_ARTICLES);

    for i in 0..NUM_ARTICLES {
        let client_clone = client.clone();
        let db_conn_clone = db_conn.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        let progress_clone = progress.clone();

        let task_index = i + 1;

        let task = tokio::spawn(async move {
            let permit = semaphore_clone.acquire_owned().await.expect("Nie udało się uzyskać pozwolenia z semafora");

            let result: Result<Option<Article>> = async {
                let article_opt = scrape_random_article(&client_clone).await?;
                tokio::time::sleep(Duration::from_millis(DELAY_BETWEEN_BATCHES_MS)).await;
                Ok(article_opt)
            }.await;

            let mut saved_this_task = false;
            match result {
                Ok(Some(article)) => {
                    match save_article(&db_conn_clone, &article).await {
                        Ok(true) => {
                            info!("Zapisano artykuł ({}): {} - {}", task_index, article.title, article.url);
                            saved_this_task = true;
                            progress_clone.increment_saved().await;
                        }
                        Ok(false) => {
                            info!("Artykuł już istnieje (zignorowano) ({}): {}", task_index, article.url);
                        }
                        Err(e) => {
                            error!("Błąd zapisu do bazy danych dla {} (iteracja {}): {}", article.url, task_index, e);
                        }
                    }
                }
                Ok(None) => {
                    warn!("Pominięto artykuł (brak danych) - iteracja {}", task_index);
                }
                Err(e) => {
                    error!("Błąd podczas scrapowania artykułu (iteracja {}): {}", task_index, e);
                }
            }

            let completed = progress_clone.increment_completed().await;
            progress_clone.print_progress(completed).await;

            drop(permit);
            saved_this_task
        });
        tasks.push(task);
    }

    println!("Oczekiwanie na zakończenie wszystkich zadań...");
    let results = join_all(tasks).await;

    let mut articles_saved_count = 0;
    for result in results {
        match result {
            Ok(saved) => {
                if saved {
                    articles_saved_count += 1;
                }
            }
            Err(e) => {
                error!("Task zakończony błędem (panika lub anulowanie): {}", e);
            }
        }
    }

    let final_elapsed = progress.start_time.elapsed();
    println!("\n=== PODSUMOWANIE ===");
    println!("Zakończono scrapowanie!");
    println!("Zapisano {} nowych artykułów", articles_saved_count);
    println!("Całkowity czas: {:.1} sekund", final_elapsed.as_secs_f64());
    println!("Średnia szybkość: {:.1} artykułów/sekundę", NUM_ARTICLES as f64 / final_elapsed.as_secs_f64());

    info!("Zakończono scrapowanie. Zapisano {} nowych artykułów.", articles_saved_count);
    Ok(())
}