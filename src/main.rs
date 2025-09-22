use tokio_cron_scheduler::{Job, JobScheduler};

const PROMPT: &str = include_str!("../assets/prompt.txt");
const REQUEST: &str = include_str!("../assets/request.json");

#[derive(serde::Deserialize, serde::Serialize)]
struct Settings {
    emails: Vec<String>,
    gemini_key: String,
}

impl Settings {
    async fn load() -> Result<Self, String> {
        let content = tokio::fs::read_to_string("settings.toml")
            .await
            .map_err(|e| format!("Failed to read settings file: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;

    println!("Starting daily knowledge job scheduler...");
    daily_knowledge_job().await.unwrap();

    sched
        .add(Job::new_async("0 0 0 * * *", |_, _| {
            Box::pin(async move {
                daily_knowledge_job().await.unwrap();
            })
        })?)
        .await?;

    sched.start().await?;
    tokio::signal::ctrl_c().await?;

    Ok(())
}

#[derive(serde::Deserialize, Debug)]
pub struct AiResponse {
    pub title: String,
    pub category: String,
    pub content: String,
}

async fn get_ai_response(settings: &Settings) -> Result<AiResponse, String> {
    let response = reqwest::Client::new()
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent")
        .header(
            "Content-Type",
            "application/json",
        )
        .header(
            "X-goog-api-key",
            settings.gemini_key.clone(),
        )
        .body(
            REQUEST.replace("<prompt>", PROMPT),
        )
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to get AI response: {}", e))?;

    let response: serde_json::Value = serde_json::from_str(&response)
        .map_err(|e| format!("Failed to deserialize AI response: {}", e))?;

    let response = response
        .pointer("/candidates/0/content/parts/0/text")
        .ok_or_else(|| format!("Failed to get AI response: {}", response))?
        .as_str()
        .unwrap();

    serde_json::from_str(&response).map_err(|e| format!("Failed to deserialize AI response: {}", e))
}

async fn daily_knowledge_job() -> Result<(), String> {
    let settings = Settings::load().await?;
    let emails = &settings.emails;

    println!("Emails: {:?}", emails);

    let response = get_ai_response(&settings).await?;
    println!("Response: {:?}", response);

    Ok(())
}
