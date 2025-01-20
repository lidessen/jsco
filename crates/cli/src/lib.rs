use clap::Parser;
use jsco::jsco;
use jsco::report::Reports;
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
  /// JavaScript files, URLs, or glob patterns to check
  #[arg(required = true)]
  inputs: Vec<String>,

  /// Output format: console or json
  #[arg(short, long, default_value = "console")]
  format: String,
}

pub async fn run(arguments: Vec<String>) {
  let _ = CLIENT.get_or_init(|| Client::new());
  let _ = ALLOCATOR.get_or_init(|| Arc::new(Allocator::default()));

  let args = Args::parse_from(arguments);
  let inputs = args.inputs;

  let output_format = match args.format.to_lowercase().as_str() {
    "json" => OutputFormat::Json,
    _ => OutputFormat::HTML,
  };

  let cache_dir = PathBuf::from(CACHE_DIR);
  if !cache_dir.exists() {
    let _ = fs::create_dir(&cache_dir);
  }

  let reports = jsco(inputs).await;
  reports.output(output_format);
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
  HTML,
  Json,
}

pub trait ReportOutput {
  fn output(&self, format: OutputFormat);
}

impl ReportOutput for Reports {
  fn output(&self, format: OutputFormat) {
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
                details[open] summary ~ * {
                  animation: sweep .3s ease-in-out;
                }
                @keyframes sweep {
                  0%    {opacity: 0; transform: translateY(-10px)}
                  100%  {opacity: 1; transform: translateY(0)}
                }
                .feature-card {
                  transition: all 0.2s ease-in-out;
                }
                .feature-card:hover {
                  transform: translateY(-2px);
                  box-shadow: 0 8px 24px -12px rgba(0, 0, 0, 0.15);
                }
                "#)
              }
            }
            body class="min-h-screen p-4 md:p-8 bg-gradient-to-br from-slate-50 to-slate-100/50" {
              div class="max-w-6xl mx-auto" {
                div class="text-center mb-16" {
                  h1 class="text-4xl font-bold text-slate-900 mb-4 bg-clip-text text-transparent bg-gradient-to-r from-blue-600 to-blue-800" {
                    "JavaScript Compatibility Report"
                  }
                  p class="text-lg text-slate-600 max-w-2xl mx-auto" {
                    "Compatibility analysis based on MDN browser-compat-data"
                  }
                }

                @for report in self {
                  @if !report.found_features.is_empty() {
                    div class="space-y-8 mb-12" {
                      h3 class="text-lg font-medium text-slate-700 mb-6 pb-2 border-b border-slate-200" {
                        span class="inline-flex items-center gap-2" {
                          svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5 text-slate-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" {
                            path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {}
                            path d="M13 2v7h7" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {}
                          }
                          (report.path)
                        }
                      }
                      @for feature in &report.found_features {
                        div class="feature-card bg-white rounded-xl shadow-sm border border-slate-200/60 p-6 transition-all" {
                          div class="flex flex-col md:flex-row md:items-start md:justify-between mb-6" {
                            div {
                              h2 class="text-xl font-semibold text-blue-600/90 mb-3" {
                                (format!("{:?}", feature.feat_type))
                              }
                              a class="text-sm text-slate-500 hover:text-blue-500 hover:underline inline-flex items-center gap-1.5 group"
                                href=(feature.mdn_url) target="_blank" rel="noopener" {
                                svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 transition-transform group-hover:translate-x-0.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" {
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
                                div class="inline-flex items-center px-3 py-1.5 rounded-full text-sm bg-blue-50 text-blue-700 border border-blue-100 shadow-sm hover:bg-blue-100 transition-colors" {
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
                                  div class="rounded-lg border border-slate-200 overflow-hidden transition-all hover:border-slate-300" {
                                    div class="flex items-center justify-between px-4 py-2.5 bg-slate-50 text-sm text-slate-600 border-b border-slate-200" {
                                      span class="font-medium" {
                                        (format!("Lines {}-{}", span.start, span.end))
                                      }
                                    }
                                    div class="code-block p-4 font-mono text-sm overflow-x-auto" {
                                      code {(span.source_text(&report.source_code))}
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
