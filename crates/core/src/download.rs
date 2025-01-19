use futures_util::StreamExt;
use reqwest::Client;
use std::{
  fs::{self},
  io::{stdout, Write},
  path::PathBuf,
  sync::OnceLock,
};

const CACHE_DIR: &str = ".jsco-cache";
static CLIENT: OnceLock<Client> = OnceLock::new();

async fn get_cached_content(key: &str) -> Option<String> {
  let cache_dir = PathBuf::from(CACHE_DIR);
  if !cache_dir.exists() {
    fs::create_dir(&cache_dir).ok()?;
  }

  let cache_file = cache_dir.join(key);
  if cache_file.exists() {
    fs::read_to_string(cache_file).ok()
  } else {
    None
  }
}

async fn save_to_cache(key: &str, content: &str) -> Result<(), std::io::Error> {
  let cache_dir = PathBuf::from(CACHE_DIR);
  if !cache_dir.exists() {
    fs::create_dir(&cache_dir)?;
  }

  let cache_file = cache_dir.join(key);
  fs::write(cache_file, content)?;
  Ok(())
}

pub async fn download_with_progress(
  url: String,
  cache_key: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  if let Some(cached) = get_cached_content(&cache_key).await {
    println!("Using cached version of {}", url);
    return Ok(cached);
  }

  let res = CLIENT
    .get_or_init(|| Client::new())
    .get(&url)
    .send()
    .await?;
  let total_size = res.content_length().unwrap_or(0);
  let mut downloaded = 0;
  let mut content = String::new();

  print!("Preparing to download {}...\n", url);
  stdout().flush()?;

  let mut stream = res.bytes_stream();
  while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    downloaded += chunk.len() as u64;
    content.push_str(&String::from_utf8_lossy(&chunk));

    if total_size > 0 {
      let progress = (downloaded as f64 / total_size as f64) * 100.0;
      let width = 40;
      let filled = (width as f64 * progress / 100.0) as usize;
      let empty = width - filled;
      print!(
        "\r{} {:3.1}% [{:█<filled$}{:⋅<empty$}]",
        "Downloading",
        progress,
        "",
        "",
        filled = filled,
        empty = empty
      );
      stdout().flush()?;
    }
  }
  println!("\nDownload completed!");

  save_to_cache(&cache_key, &content).await?;
  Ok(content)
}
