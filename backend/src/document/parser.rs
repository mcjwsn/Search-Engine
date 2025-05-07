#[derive(Debug)]

pub struct Document {
    pub id: String,
    pub title: String,
    pub text: String,
    pub authors: Vec<String>,
}
pub fn parse_cisi_documents(content: &str) -> Vec<Document> {
    let mut documents = Vec::new();
    let mut current_id = String::new();
    let mut current_title = String::new();
    let mut current_text = String::new();
    let mut current_authors = Vec::new();

    let mut section = "";

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(".I") {
            // Save previous document if any
            if !current_id.is_empty() {
                documents.push(Document {
                    id: current_id.clone(),
                    title: current_title.trim().to_string(),
                    text: current_text.trim().to_string(),
                    authors: current_authors.clone(),
                });
                current_title.clear();
                current_text.clear();
                current_authors.clear();
            }
            current_id = line[2..].trim().to_string();
            section = "";
        } else if line.starts_with(".T") {
            section = "T";
        } else if line.starts_with(".A") {
            section = "A";
        } else if line.starts_with(".W") {
            section = "W";
        } else {
            match section {
                "T" => {
                    current_title.push_str(line);
                    current_title.push('\n');
                }
                "A" => {
                    current_authors.push(line.to_string());
                }
                "W" => {
                    current_text.push_str(line);
                    current_text.push('\n');
                }
                _ => {}
            }
        }
    }

    // Add last document
    if !current_id.is_empty() {
        documents.push(Document {
            id: current_id,
            title: current_title.trim().to_string(),
            text: current_text.trim().to_string(),
            authors: current_authors,
        });
    }

    documents
}
