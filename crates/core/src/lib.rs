use console::style;
use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
use md5;
use std::{
  fs,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

use download::download_with_progress;
use report::{Report, Reports};
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
  Directory(PathBuf),
  Glob(String),
}

impl InputType {
  fn from_str(s: &str) -> Self {
    if s.starts_with("http://") || s.starts_with("https://") {
      Self::Url(s.to_string())
    } else if s.contains('*') {
      Self::Glob(s.to_string())
    } else if PathBuf::from(s).is_dir() {
      Self::Directory(PathBuf::from(s))
    } else {
      Self::File(PathBuf::from(s))
    }
  }
}

pub async fn jsco(inputs: Vec<String>) -> Reports {
  let cache_dir = PathBuf::from(CACHE_DIR);
  if !cache_dir.exists() {
    let _ = fs::create_dir(&cache_dir);
  }

  println!(
    "\n{} Starting JavaScript compatibility analysis...",
    style("🔍").bold()
  );

  let (tx, mut rx) = mpsc::channel(32);
  let (count_tx, mut count_rx) = mpsc::channel(32);
  let count_tx_clone = count_tx.clone();
  let inputs_clone = inputs.clone();

  // First pass to count total files
  let count_handle = tokio::spawn(async move {
    let mut total = 0;
    for input in &inputs {
      let input_type = InputType::from_str(input);
      match input_type {
        InputType::File(_) => total += 1,
        InputType::Url(_) => total += 1,
        InputType::Directory(dir) => {
          if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
              if let Ok(entry) = entry {
                if entry.path().extension().unwrap_or_default() == "js" {
                  total += 1;
                }
              }
            }
          }
        }
        InputType::Glob(pattern) => {
          if let Ok(paths) = glob(&pattern) {
            for entry in paths {
              if let Ok(path) = entry {
                if path.extension().unwrap_or_default() == "js" {
                  total += 1;
                }
              }
            }
          }
        }
      }
    }
    count_tx.send(total).await.ok();
  });

  // Wait for count
  let total_files = count_rx.recv().await.unwrap_or(0);
  let progress = ProgressBar::new(total_files);
  progress.set_style(
    ProgressStyle::default_bar()
      .template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%)",
      )
      .unwrap()
      .progress_chars("#>-"),
  );

  let download_handle = tokio::spawn(async move {
    for input in inputs_clone {
      let input_type = InputType::from_str(&input);
      match input_type {
        InputType::File(path) => {
          if let Ok(content) = fs::read_to_string(&path) {
            tx.send((path.to_string_lossy().to_string(), content))
              .await
              .ok();
            count_tx_clone.send(1).await.ok();
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

          if let Ok(content) = download_with_progress(url_with_timestamp, get_cache_key(&url)).await
          {
            tx.send((url.to_string(), content)).await.ok();
            count_tx_clone.send(1).await.ok();
          }
        }
        InputType::Directory(dir) => {
          println!(
            "\n{} Scanning directory: {}",
            style("📁").bold(),
            style(&dir.display()).cyan()
          );
          let files = fs::read_dir(dir).unwrap();
          for file in files {
            if let Ok(file) = file {
              let path = file.path();
              if path.extension().unwrap_or_default() == "js" {
                if let Ok(content) = fs::read_to_string(&path) {
                  tx.send((path.to_string_lossy().to_string(), content))
                    .await
                    .ok();
                  count_tx_clone.send(1).await.ok();
                }
              }
            }
          }
        }
        InputType::Glob(pattern) => {
          println!(
            "\n{} Scanning files matching: {}",
            style("🔍").bold(),
            style(&pattern).cyan()
          );
          if let Ok(paths) = glob(&pattern) {
            for entry in paths {
              if let Ok(path) = entry {
                if path.extension().unwrap_or_default() == "js" {
                  if let Ok(content) = fs::read_to_string(&path) {
                    tx.send((path.to_string_lossy().to_string(), content))
                      .await
                      .ok();
                    count_tx_clone.send(1).await.ok();
                  }
                }
              }
            }
          }
        }
      }
    }
  });

  let mut collector = Vec::new();
  while let Some((path, source_code)) = rx.recv().await {
    let mut report = Report::new(path.clone(), source_code);
    report.check_feature();
    report.prepare_output();

    if let Some(_) = count_rx.recv().await {
      progress.inc(1);
      let has_features = !report.found_features.is_empty();
      let feature_count = report.found_features.len();
      if has_features {
        progress.println(format!(
          "{} {} - Found {} features",
          style("✓").green(),
          style(&path).cyan(),
          style(feature_count).yellow()
        ));
      }
    }

    collector.push(report);
  }

  progress.finish_with_message("Analysis complete!");
  count_handle.await.ok();
  download_handle.await.ok();

  println!("\n{} Analysis Summary:", style("📊").bold());
  println!("  {} Total files processed", style(total_files).cyan());
  println!(
    "  {} Files with features",
    style(
      collector
        .iter()
        .filter(|r| !r.found_features.is_empty())
        .count()
    )
    .green()
  );
  println!(
    "  {} Total features found",
    style(
      collector
        .iter()
        .map(|r| r.found_features.len())
        .sum::<usize>()
    )
    .yellow()
  );
  println!("");

  collector
}
