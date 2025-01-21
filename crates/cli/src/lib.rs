use browserslist::{execute, Distrib, Opts};
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
        let browsers = execute(&Opts::default()).unwrap_or_default();
        let mut chrome_versions = Vec::new();
        let mut firefox_versions = Vec::new();
        let mut safari_versions = Vec::new();
        let mut edge_versions = Vec::new();
        let mut other_browsers = Vec::new();
        for browser in &browsers {
          let name = browser.name().to_lowercase();
          match name.as_str() {
            "chrome" | "and_chr" | "chrome android" => {
              chrome_versions.push(browser.version().to_string())
            }
            "firefox" | "firefox android" => firefox_versions.push(browser.version().to_string()),
            "safari" | "ios_saf" => safari_versions.push(browser.version().to_string()),
            "edge" => edge_versions.push(browser.version().to_string()),
            _ => other_browsers.push((browser.name(), browser.version())),
          }
        }

        // Group and sort versions
        let format_versions = |versions: &[String]| -> String {
          let mut versions = versions.to_vec();
          versions.sort();
          versions.join(", ")
        };

        let default_version = "0".to_string();
        chrome_versions.sort();
        firefox_versions.sort();
        safari_versions.sort();
        edge_versions.sort();

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

                div class="mb-12 bg-white rounded-xl shadow-sm border border-slate-200/60 p-6" {
                  h2 class="text-lg font-semibold text-slate-800 mb-4" {
                    "Target Browsers"
                    span class="ml-2 text-sm font-normal text-slate-500" {
                      "(from .browserslistrc)"
                    }
                  }
                  div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4" {
                    @if !chrome_versions.is_empty() {
                      div class="p-4 rounded-lg bg-gradient-to-br from-slate-50 to-white border border-slate-200" {
                        div class="flex items-center gap-2 text-slate-700 mb-3" {
                          span class="font-medium" { "Chrome" }
                        }
                        div class="space-y-1.5" {
                          div class="text-xs font-medium uppercase tracking-wider text-slate-500" { "Required Versions" }
                          div class="text-sm text-slate-700 font-mono" {
                            @let min_version = chrome_versions.iter().min().unwrap_or(&default_version);
                            @let max_version = chrome_versions.iter().max().unwrap_or(&default_version);
                            @if min_version == max_version {
                              (format!("v{}", min_version))
                            } @else {
                              (format!("v{} - v{}", min_version, max_version))
                            }
                          }
                          div class="text-xs text-slate-500 mt-1" {
                            (format!("Including: {}", format_versions(&chrome_versions)))
                          }
                        }
                      }
                    }

                    @if !firefox_versions.is_empty() {
                      div class="p-4 rounded-lg bg-gradient-to-br from-slate-50 to-white border border-slate-200" {
                        div class="flex items-center gap-2 text-slate-700 mb-3" {
                          span class="font-medium" { "Firefox" }
                        }
                        div class="space-y-1.5" {
                          div class="text-xs font-medium uppercase tracking-wider text-slate-500" { "Required Versions" }
                          div class="text-sm text-slate-700 font-mono" {
                            @let min_version = firefox_versions.iter().min().unwrap_or(&default_version);
                            @let max_version = firefox_versions.iter().max().unwrap_or(&default_version);
                            @if min_version == max_version {
                              (format!("v{}", min_version))
                            } @else {
                              (format!("v{} - v{}", min_version, max_version))
                            }
                          }
                          div class="text-xs text-slate-500 mt-1" {
                            (format!("Including: {}", format_versions(&firefox_versions)))
                          }
                        }
                      }
                    }

                    @if !safari_versions.is_empty() {
                      div class="p-4 rounded-lg bg-gradient-to-br from-slate-50 to-white border border-slate-200" {
                        div class="flex items-center gap-2 text-slate-700 mb-3" {
                          span class="font-medium" { "Safari" }
                        }
                        div class="space-y-1.5" {
                          div class="text-xs font-medium uppercase tracking-wider text-slate-500" { "Required Versions" }
                          div class="text-sm text-slate-700 font-mono" {
                            @let min_version = safari_versions.iter().min().unwrap_or(&default_version);
                            @let max_version = safari_versions.iter().max().unwrap_or(&default_version);
                            @if min_version == max_version {
                              (format!("v{}", min_version))
                            } @else {
                              (format!("v{} - v{}", min_version, max_version))
                            }
                          }
                          div class="text-xs text-slate-500 mt-1" {
                            (format!("Including: {}", format_versions(&safari_versions)))
                          }
                        }
                      }
                    }

                    @if !edge_versions.is_empty() {
                      div class="p-4 rounded-lg bg-gradient-to-br from-slate-50 to-white border border-slate-200" {
                        div class="flex items-center gap-2 text-slate-700 mb-3" {
                          span class="font-medium" { "Edge" }
                        }
                        div class="space-y-1.5" {
                          div class="text-xs font-medium uppercase tracking-wider text-slate-500" { "Required Versions" }
                          div class="text-sm text-slate-700 font-mono" {
                            @let min_version = edge_versions.iter().min().unwrap_or(&default_version);
                            @let max_version = edge_versions.iter().max().unwrap_or(&default_version);
                            @if min_version == max_version {
                              (format!("v{}", min_version))
                            } @else {
                              (format!("v{} - v{}", min_version, max_version))
                            }
                          }
                          div class="text-xs text-slate-500 mt-1" {
                            (format!("Including: {}", format_versions(&edge_versions)))
                          }
                        }
                      }
                    }

                    @if !other_browsers.is_empty() {
                      div class="p-4 rounded-lg bg-gradient-to-br from-slate-50 to-white border border-slate-200" {
                        div class="flex items-center gap-2 text-slate-700 mb-3" {
                          span class="font-medium" { "Other" }
                        }
                        div class="space-y-2" {
                          @for (name, version) in &other_browsers {
                            div class="text-sm text-slate-600" {
                              span class="font-medium" { (name) }
                              span class="font-mono ml-2" { (format!("v{}", version)) }
                            }
                          }
                        }
                      }
                    }
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
                              @let browsers = execute(&Opts::default()).unwrap_or_default();

                              @for (browser, version) in &browser_info {
                                @let is_compatible = is_supported(browser, version, browsers.as_slice());
                                div class=(if is_compatible {
                                  "inline-flex items-center px-3 py-1.5 rounded-full text-sm bg-green-50 text-green-700 border border-green-100 shadow-sm hover:bg-green-100 transition-colors"
                                } else {
                                  "inline-flex items-center px-3 py-1.5 rounded-full text-sm bg-red-50 text-red-700 border border-red-100 shadow-sm hover:bg-red-100 transition-colors"
                                }) {
                                  span class="mr-1.5" {
                                    @if is_compatible {
                                      svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" {
                                        path d="M20 6L9 17l-5-5" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {}
                                      }
                                    } @else {
                                      svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" {
                                        path d="M18 6L6 18M6 6l12 12" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {}
                                      }
                                    }
                                  }
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

pub fn is_supported(browser: &str, version: &str, browsers: &[Distrib]) -> bool {
  // If no browsers are specified, consider it supported
  if browsers.is_empty() {
    return true;
  }

  // Get the browser name in lowercase for case-insensitive comparison
  let browser_name = browser.to_lowercase();

  // Map our internal names to browserslist names
  let matches_browser = |b: &Distrib| {
    let b_name = b.name().to_lowercase();
    match browser_name.as_str() {
      "chrome" => matches!(b_name.as_str(), "and_chr" | "chrome" | "chrome android"),
      "firefox" => matches!(b_name.as_str(), "firefox" | "firefox android"),
      "safari" => matches!(b_name.as_str(), "safari" | "ios_saf"),
      "edge" => b_name == "edge",
      _ => false,
    }
  };

  // Find matching browsers from the requirements
  let matching_browsers: Vec<_> = browsers
    .into_iter()
    .filter(|b| matches_browser(b))
    .collect();

  // If no matching browsers found in requirements, consider it supported
  if matching_browsers.is_empty() {
    return true;
  }

  // Skip if version is "true" (meaning always supported)
  if version == "true" {
    return true;
  }

  // Get our major version number
  let our_version = version.split('.').next().unwrap_or("0");
  let our_version: u32 = our_version.parse().unwrap_or(0);

  // Check against all matching browsers
  for browser in matching_browsers {
    let their_version = browser.version().split('.').next().unwrap_or("0");
    let their_version: u32 = their_version.parse().unwrap_or(0);

    // If our required version is higher than their version, it's not supported
    if our_version > their_version {
      return false;
    }
  }

  // If we got here, all browser requirements are met
  true
}
