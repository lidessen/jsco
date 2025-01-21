use oxc::span::Span;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::bcd::Compatibility;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum JsFeature {
  OptionalChaining,
  NullishCoalescing,
  PrivateField,
  PrivateMethod,
  // ES2022+
  TopLevelAwait,
  ClassStaticBlock,
  // ES2021
  LogicalAssignment,
  NumericSeparator,
  // ES2020
  BigInt,
  DynamicImport,
  // ES2019
  OptionalCatchBinding,
  // ES2018
  AsyncIteration,
  RestSpread,
  // ES2017
  Await,
  Decorator,
  ServiceWorker,
  // performance.now()
  PerformanceNow,
  // requestIdleCallback
  RequestIdleCallback,
  // TypedArray
  TypedArray,
  // Int8Array
  Int8Array,
  Uint8Array,
  Int16Array,
  Uint16Array,
  Int32Array,
  Uint32Array,
  Float32Array,
  Float64Array,
}

impl Serialize for JsFeature {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(self.key())
  }
}

pub type BrowserSupport = HashMap<String, String>;

#[derive(Debug, Clone, Serialize)]
pub struct FeatureReport {
  pub feat_type: JsFeature,
  #[serde(skip)]
  pub found_in: Vec<Span>,
  #[serde(rename = "locations")]
  pub locations: Vec<Location>,
  #[serde(serialize_with = "serialize_browser_support")]
  pub support: Arc<Mutex<BrowserSupport>>,
  pub mdn_url: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Location {
  pub start: usize,
  pub end: usize,
  pub code: String,
}

fn serialize_browser_support<S>(
  support: &Arc<Mutex<BrowserSupport>>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  support.lock().unwrap().serialize(serializer)
}

impl FeatureReport {
  pub fn new(feat_type: JsFeature, support: BrowserSupport) -> Self {
    Self {
      feat_type,
      found_in: Vec::new(),
      locations: Vec::new(),
      support: Arc::new(Mutex::new(support)),
      mdn_url: String::new(),
    }
  }

  pub fn add_span(&mut self, span: Span) {
    self.found_in.push(span);
    self.locations.push(Location {
      start: span.start as usize,
      end: span.end as usize,
      code: String::new(),
    });
  }

  pub fn prepare_output(&mut self, source_code: &str) {
    for (i, span) in self.found_in.iter().enumerate() {
      if let Some(location) = self.locations.get_mut(i) {
        location.code = span.source_text(source_code).to_string();
      }
    }
  }
}

pub trait JsFeatureTrait {
  fn compat(&self) -> Compatibility;
  fn browser_support(&self) -> BrowserSupport;
  fn mdn_url(&self) -> String;
}
