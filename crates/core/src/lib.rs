use md5;
use std::{
  fs,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

use download::download_with_progress;
use report::Report;
use tokio::sync::mpsc;

pub mod bcd;
pub mod download;
pub mod feature;
pub mod report;

const CACHE_DIR: &str = ".jsco-cache";

fn get_cache_key(url: &str) -> String {
  if let Ok(parsed_url) = Url::parse(url) {
    let base_url = format!(
      "{}://{}{}",
      parsed_url.scheme(),
      parsed_url.host_str().unwrap_or(""),
      parsed_url.path()
    );
    format!("{:x}", md5::compute(base_url))
  } else {
    format!("{:x}", md5::compute(url))
  }
}

#[derive(Debug)]
enum InputType {
  File(PathBuf),
  Url(String),
}

impl InputType {
  fn from_str(s: &str) -> Self {
    if s.starts_with("http://") || s.starts_with("https://") {
      Self::Url(s.to_string())
    } else {
      Self::File(PathBuf::from(s))
    }
  }
}

pub async fn jsco(input: &str) -> Option<Report> {
  let input_type = InputType::from_str(input);

  let cache_dir = PathBuf::from(CACHE_DIR);
  if !cache_dir.exists() {
    let _ = fs::create_dir(&cache_dir);
  }

  let (tx, mut rx) = mpsc::channel(32);
  let input_str = input.to_string();

  let download_handle = tokio::spawn(async move {
    match input_type {
      InputType::File(path) => {
        if let Ok(content) = fs::read_to_string(&path) {
          tx.send(content).await.ok();
        }
      }
      InputType::Url(url) => {
        let url_with_timestamp = if url.contains('?') {
          url.clone()
        } else {
          let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
          format!("{}?t={}", url, timestamp)
        };

        if let Ok(content) = download_with_progress(url_with_timestamp, get_cache_key(&url)).await {
          tx.send(content).await.ok();
        }
      }
    }
  });

  let mut collector = None;
  if let Some(source_code) = rx.recv().await {
    let mut report = Report::new(input_str.clone(), source_code.clone());
    report.check_feature();
    report.prepare_output();
    collector = Some(report);
  }

  download_handle.await.ok();
  collector
}
