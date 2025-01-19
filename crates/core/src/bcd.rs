use crate::{
  download::download_with_progress,
  feature::{BrowserSupport, JsFeature, JsFeatureTrait},
};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use serde_json;
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

#[derive(Debug, Deserialize, Clone, Serialize)]
#[allow(dead_code)]
pub struct Compatibility {
  #[serde(default)]
  description: Option<String>,
  #[serde(default)]
  mdn_url: Option<String>,
  status: Status,
  support: HashMap<String, VersionSupport>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[allow(dead_code)]
pub struct Status {
  deprecated: bool,
  experimental: bool,
  standard_track: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub enum VersionSupport {
  Single(SupportInfo),
  Multiple(Vec<SupportInfo>),
  Unknown(serde_json::Value),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[allow(dead_code)]
pub struct SupportInfo {
  version_added: VersionAdded,

  #[serde(flatten)]
  extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub enum VersionAdded {
  Boolean(bool),
  String(String),
  Null,
}

static BCD_DATA: OnceCell<Arc<serde_json::Value>> = OnceCell::new();
static FEATURE_COMPAT_CACHE: Lazy<HashMap<JsFeature, OnceCell<Compatibility>>> = Lazy::new(|| {
  let mut cache = HashMap::new();
  for feature in [
    JsFeature::OptionalChaining,
    JsFeature::NullishCoalescing,
    JsFeature::PrivateField,
    JsFeature::PrivateMethod,
    JsFeature::TopLevelAwait,
    JsFeature::ClassStaticBlock,
    JsFeature::LogicalAssignment,
    JsFeature::NumericSeparator,
    JsFeature::BigInt,
    JsFeature::DynamicImport,
    JsFeature::OptionalCatchBinding,
    JsFeature::AsyncIteration,
    JsFeature::RestSpread,
    JsFeature::Await,
    JsFeature::Decorator,
  ] {
    cache.insert(feature, OnceCell::new());
  }
  cache
});

const BCD_CACHE_FILE: &str = ".jsco-cache/browser-compat-data.json";
const FEATURE_CACHE_DIR: &str = ".jsco-cache/features";

fn ensure_cache_dir(dir: &str) -> std::io::Result<PathBuf> {
  let cache_dir = PathBuf::from(dir);
  if !cache_dir.exists() {
    fs::create_dir_all(&cache_dir)?;
  }
  Ok(cache_dir)
}

async fn download_bcd_data_async() -> Arc<serde_json::Value> {
  if let Ok(data) = fs::read_to_string(BCD_CACHE_FILE) {
    println!("Using cached BCD data");
    if let Ok(parsed_data) = serde_json::from_str(&data) {
      return Arc::new(parsed_data);
    }
  }

  let data = download_with_progress(
    "https://cdn.jsdelivr.net/npm/@mdn/browser-compat-data/data.json".to_string(),
    "browser-compat-data.json".to_string(),
  )
  .await
  .expect("Failed to download BCD data");

  let parsed_data: serde_json::Value =
    serde_json::from_str(&data).expect("Failed to parse bcd data");
  Arc::new(parsed_data)
}

fn download_bcd_data() -> &'static Arc<serde_json::Value> {
  BCD_DATA.get_or_init(|| {
    tokio::task::block_in_place(|| {
      tokio::runtime::Handle::current().block_on(download_bcd_data_async())
    })
  })
}

fn get_version_added(support: Option<&VersionSupport>) -> Option<String> {
  match support {
    Some(VersionSupport::Single(single)) => match single.version_added.clone() {
      VersionAdded::Boolean(true) => Some("true".to_string()),
      VersionAdded::Boolean(false) => Some("false".to_string()),
      VersionAdded::String(version) => Some(version),
      VersionAdded::Null => None,
    },
    Some(VersionSupport::Multiple(multiple)) => {
      multiple
        .first()
        .and_then(|c| match c.version_added.clone() {
          VersionAdded::Boolean(true) => Some("true".to_string()),
          VersionAdded::Boolean(false) => Some("false".to_string()),
          VersionAdded::String(version) => Some(version),
          VersionAdded::Null => None,
        })
    }
    Some(VersionSupport::Unknown(value)) => {
      println!("Unknown version added: {:?}", value);
      None
    }
    None => None,
  }
}

impl JsFeatureTrait for JsFeature {
  fn compat(&self) -> Compatibility {
    let compat = FEATURE_COMPAT_CACHE[self].get_or_init(|| {
      // Try to load from feature cache first
      let cache_file =
        PathBuf::from(FEATURE_CACHE_DIR).join(format!("{}.json", self.key().replace('.', "_")));
      if let Ok(data) = fs::read_to_string(&cache_file) {
        if let Ok(compat) = serde_json::from_str(&data) {
          return compat;
        }
      }

      // If not in cache, get from BCD data
      let bcd = download_bcd_data();
      let feature = bcd
        .read_from_path(self.key())
        .expect(&format!("Feature {} not found", self.key()))
        .clone();
      let compat: Compatibility =
        serde_json::from_value(feature.get("__compat").unwrap().clone()).unwrap();

      // Save to feature cache
      let _ = ensure_cache_dir(FEATURE_CACHE_DIR);
      let _ = fs::write(cache_file, serde_json::to_string(&compat).unwrap());

      compat
    });

    compat.clone()
  }

  fn browser_support(&self) -> BrowserSupport {
    let compat = self.compat();

    let mut support = HashMap::new();
    if let Some(version) = get_version_added(compat.support.get("chrome")) {
      support.insert("chrome".to_string(), version);
    }
    if let Some(version) = get_version_added(compat.support.get("firefox")) {
      support.insert("firefox".to_string(), version);
    }
    if let Some(version) = get_version_added(compat.support.get("safari")) {
      support.insert("safari".to_string(), version);
    }
    if let Some(version) = get_version_added(compat.support.get("edge")) {
      support.insert("edge".to_string(), version);
    }

    support
  }

  fn mdn_url(&self) -> String {
    let compat = self.compat();
    compat.mdn_url.unwrap_or_default()
  }
}

impl JsFeature {
  pub fn key(&self) -> &str {
    match self {
      JsFeature::OptionalChaining => "javascript.operators.optional_chaining",
      JsFeature::NullishCoalescing => "javascript.operators.nullish_coalescing",
      JsFeature::PrivateField => "javascript.classes.private_class_fields",
      JsFeature::PrivateMethod => "javascript.classes.private_class_methods",
      JsFeature::TopLevelAwait => "javascript.statements.top_level_await",
      JsFeature::ClassStaticBlock => "javascript.classes.class_static_block",
      JsFeature::LogicalAssignment => "javascript.operators.logical_assignment_operators",
      JsFeature::NumericSeparator => "javascript.operators.numeric_separators",
      JsFeature::BigInt => "javascript.builtins.bigint",
      JsFeature::DynamicImport => "javascript.operators.import",
      JsFeature::OptionalCatchBinding => "javascript.operators.optional_chaining",
      JsFeature::AsyncIteration => "javascript.builtins.AsyncIterator",
      JsFeature::RestSpread => "javascript.operators.spread",
      JsFeature::Await => "javascript.operators.await",
      JsFeature::Decorator => "javascript.builtins.decorators",
    }
  }
}

pub trait JsonRead {
  fn read_from_path(&self, path: &str) -> Option<&serde_json::Value>;
}

impl JsonRead for serde_json::Value {
  // read data from deep structure
  fn read_from_path(&self, path: &str) -> Option<&serde_json::Value> {
    let parts = path.split('.').collect::<Vec<&str>>();
    let mut current = self;
    for part in parts {
      current = current.get(part)?;
    }
    Some(current)
  }
}
