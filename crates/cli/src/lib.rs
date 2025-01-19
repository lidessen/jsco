use clap::Parser;
use jsco::jsco;
use jsco::report::Report;
use maud::html;
use oxc::allocator::Allocator;
use reqwest::Client;
use std::io::Write;
use std::sync::OnceLock;
use std::{
  fs::{self},
  path::PathBuf,
  sync::Arc,
};

const CACHE_DIR: &str = ".jsco-cache";

static CLIENT: OnceLock<Client> = OnceLock::new();
static ALLOCATOR: OnceLock<Arc<Allocator>> = OnceLock::new();

#[derive(Parser, Debug)]
#[command(version, about = "JavaScript Compatibility Checker")]
struct Args {
  /// JavaScript file or URL to check
  #[arg(index = 1)]
  input: String,

  /// Output format: console or json
  #[arg(short, long, default_value = "console")]
  format: String,
}

pub async fn run(arguments: Vec<String>) {
  let _ = CLIENT.get_or_init(|| Client::new());
  let _ = ALLOCATOR.get_or_init(|| Arc::new(Allocator::default()));

  let args = Args::parse_from(arguments);
  let input = args.input.clone();

  let output_format = match args.format.to_lowercase().as_str() {
    "json" => OutputFormat::Json,
    _ => OutputFormat::HTML,
  };

  let cache_dir = PathBuf::from(CACHE_DIR);
  if !cache_dir.exists() {
    let _ = fs::create_dir(&cache_dir);
  }

  let report = jsco(input.as_str()).await;
  if let Some(report) = report {
    report.output(output_format, &report.source_code);
  }
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
  HTML,
  Json,
}

pub trait ReportOutput {
  fn output(&self, format: OutputFormat, source_code: &str);
}

impl ReportOutput for Report {
  fn output(&self, format: OutputFormat, source_code: &str) {
    match format {
      OutputFormat::HTML => {
        let html_output = html! {
          html class="bg-slate-50" {
            head {
              title { "JavaScript Compatibility Report" }
              script src="https://cdn.tailwindcss.com" {}
              meta charset="UTF-8" {}
              meta name="viewport" content="width=device-width, initial-scale=1.0" {}
              style type="text/css" {
                (r#"
                summary {
                  list-style: none;
                }
                summary::-webkit-details-marker {
                  display: none;
                }
                summary::marker {
                  display: none;
                }
                .code-block {
                  background-image: linear-gradient(to bottom, #f8fafc, #f1f5f9);
                }
                "#)
              }
            }
            body class="min-h-screen p-4 md:p-8" {
              div class="max-w-6xl mx-auto" {
                div class="text-center mb-12" {
                  h1 class="text-4xl font-bold text-slate-900 mb-4" {
                    "JavaScript Compatibility Report"
                  }
                  p class="text-lg text-slate-600" {
                    "Compatibility analysis based on MDN browser-compat-data"
                  }
                }

                div class="space-y-8" {
                  @for feature in &self.found_features {
                    div class="bg-white rounded-xl shadow-sm border border-slate-200/60 p-6 transition-all hover:shadow-md" {
                      div class="flex flex-col md:flex-row md:items-start md:justify-between mb-6" {
                        div {
                          h2 class="text-xl font-semibold text-blue-600/90 mb-2" {
                            (format!("{:?}", feature.feat_type))
                          }
                          a class="text-sm text-slate-500 hover:text-blue-500 hover:underline inline-flex items-center gap-1"
                            href=(feature.mdn_url) target="_blank" rel="noopener" {
                            svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" {
                              path d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {}
                            }
                            span { "MDN Documentation" }
                          }
                        }
                        div class="flex flex-wrap gap-2 mt-4 md:mt-0" {
                          @let support = feature.support.lock().unwrap();
                          @let mut browser_info: Vec<_> = support.iter().collect();
                          @let _ = browser_info.sort_by(|a, b| a.0.cmp(b.0));

                          @for (browser, version) in &browser_info {
                            div class="inline-flex items-center px-3 py-1.5 rounded-full text-sm bg-blue-50 text-blue-700 border border-blue-100 shadow-sm" {
                              (format!("{} ≥ {}", browser, version))
                            }
                          }
                        }
                      }

                      div class="mt-6" {
                        details class="group" {
                          summary class="text-lg font-medium text-slate-800 cursor-pointer hover:text-blue-600 transition-colors" {
                            span class="inline-flex items-center gap-2" {
                              svg xmlns="http://www.w3.org/2000/svg"
                                class="w-5 h-5 text-slate-400 group-open:rotate-90 transition-transform"
                                viewBox="0 0 24 24" fill="none" stroke="currentColor" {
                                path d="M9 5l7 7-7 7" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {}
                              }
                              (format!("Found in {} locations", feature.found_in.len()))
                            }
                          }
                          div class="space-y-4 mt-4" {
                            @for span in &feature.found_in {
                              div class="rounded-lg border border-slate-200 overflow-hidden" {
                                div class="flex items-center justify-between px-4 py-2 bg-slate-50 text-sm text-slate-600 border-b border-slate-200" {
                                  span class="font-medium" {
                                    (format!("Lines {}-{}", span.start, span.end))
                                  }
                                }
                                div class="code-block p-4 font-mono text-sm overflow-x-auto" {
                                  code {(span.source_text(source_code))}
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        };

        // Create output directory if it doesn't exist
        let output_dir = "jsco-output";
        let _ = fs::create_dir_all(output_dir);

        // Generate timestamp for the filename
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let output_file = format!("{}/report_{}.html", output_dir, timestamp);

        // Write the HTML to file
        if let Ok(mut file) = fs::File::create(&output_file) {
          if let Ok(_) = file.write_all(html_output.into_string().as_bytes()) {
            println!("Report saved to: {}", output_file);
          } else {
            eprintln!("Failed to write report to file");
          }
        } else {
          eprintln!("Failed to create output file");
        }

        // open the file
        let _ = open::that(output_file);
      }

      OutputFormat::Json => {
        if let Ok(json) = serde_json::to_string_pretty(self) {
          let output_dir = "jsco-output";
          let _ = fs::create_dir_all(output_dir);

          let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
          let output_file = format!("{}/report_{}.json", output_dir, timestamp);

          if let Ok(mut file) = fs::File::create(&output_file) {
            if let Ok(_) = file.write_all(json.as_bytes()) {
              println!("Report saved to: {}", output_file);
            } else {
              eprintln!("Failed to write report to file");
            }
          } else {
            eprintln!("Failed to create output file");
          }
        } else {
          eprintln!("Failed to serialize report to JSON");
        }
      }
    }
  }
}
