#![deny(clippy::all)]

use jsco as core;
use napi::bindgen_prelude::*;

#[macro_use]
extern crate napi_derive;

#[napi]
pub async fn jsco(source_code: String) -> Result<serde_json::Value> {
  let report = core::jsco(vec![source_code.into()]).await;
  Ok(serde_json::to_value(report).unwrap())
}

#[napi]
pub async fn run(args: Vec<String>) -> () {
  jsco_cli::run(args).await;
}
