use oxc::allocator::Allocator;
use oxc::ast::ast::ArrayExpressionElement;
use oxc::ast::ast::Expression;
use oxc::ast::ast::MemberExpression;
use oxc::ast::AstKind;
use oxc::diagnostics::OxcDiagnostic;
use oxc::parser::Parser;
use oxc::span::SourceType;
use oxc::span::Span;
use oxc_semantic::SemanticBuilder;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::feature::{BrowserSupport, FeatureReport, JsFeature, JsFeatureTrait};

#[derive(Debug, Clone, Serialize)]
pub struct Report {
  #[serde(serialize_with = "serialize_browser_support")]
  pub browser_support: Arc<Mutex<BrowserSupport>>,
  #[serde(skip)]
  pub features: Arc<Mutex<HashMap<JsFeature, FeatureReport>>>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub found_features: Vec<FeatureReport>,
  pub path: String,
  pub source_code: String,
}

pub type Reports = Vec<Report>;

fn serialize_browser_support<S>(
  browser_support: &Arc<Mutex<BrowserSupport>>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  browser_support.lock().unwrap().serialize(serializer)
}

impl Report {
  pub fn new(path: String, source_code: String) -> Self {
    Self {
      features: Arc::new(Mutex::new(HashMap::new())),
      browser_support: Arc::new(Mutex::new(BrowserSupport::default())),
      found_features: Vec::new(),
      path,
      source_code,
    }
  }

  pub fn prepare_output(&mut self) {
    self.found_features = self.features.lock().unwrap().values().cloned().collect();
    for feature in &mut self.found_features {
      feature.prepare_output(&self.source_code);
    }
  }

  pub fn check_feature(&self) {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(&self.path).unwrap();
    let ret = Parser::new(&allocator, &self.source_code, source_type).parse();

    if !ret.errors.is_empty() {
      println!("Failed to parse JavaScript code");
      for error in ret.errors {
        println!("Error: {}", error);
      }
      return;
    }

    let semantic_ret = SemanticBuilder::new().build(&ret.program);
    let errors: Vec<OxcDiagnostic> = vec![];

    for node in semantic_ret.semantic.nodes() {
      match node.kind() {
        AstKind::LogicalExpression(it) if it.operator.as_str() == "??" => {
          self.process_found(JsFeature::NullishCoalescing, it.span);
        }
        AstKind::ChainExpression(it) => {
          self.process_found(JsFeature::OptionalChaining, it.span);
        }
        AstKind::ClassBody(it) => {
          for prop in it.body.iter() {
            if let Some(key) = prop.property_key() {
              match key {
                oxc::ast::ast::PropertyKey::PrivateIdentifier(ident) => {
                  if prop.is_property() {
                    self.process_found(JsFeature::PrivateField, ident.span);
                  } else {
                    self.process_found(JsFeature::PrivateMethod, ident.span);
                  }
                }
                _ => {}
              }
            }
          }
        }
        AstKind::AwaitExpression(it) => {
          self.process_found(JsFeature::Await, it.span);
        }
        AstKind::AssignmentExpression(it) => match it.operator.as_str() {
          "&&=" | "||=" | "??=" => {
            self.process_found(JsFeature::LogicalAssignment, it.span);
          }
          _ => {}
        },
        AstKind::NumericLiteral(it) => {
          if it.value.to_string().contains('_') {
            self.process_found(JsFeature::NumericSeparator, it.span);
          }
        }
        AstKind::ImportExpression(it) => {
          self.process_found(JsFeature::DynamicImport, it.span);
        }
        AstKind::CatchClause(it) => {
          if it.param.is_none() {
            self.process_found(JsFeature::OptionalCatchBinding, it.span);
          }
        }
        AstKind::ForOfStatement(it) => {
          if matches!(it.r#await, true) {
            self.process_found(JsFeature::AsyncIteration, it.span);
          }
        }
        AstKind::SpreadElement(it) => {
          self.process_found(JsFeature::RestSpread, it.span);
        }
        AstKind::ObjectExpression(obj) => {
          for prop in &obj.properties {
            if prop.is_spread() {
              self.process_found(JsFeature::RestSpread, obj.span);
            }
          }
        }
        AstKind::ArrayExpression(arr) => {
          for elem in &arr.elements {
            match elem {
              ArrayExpressionElement::SpreadElement(spread) => {
                self.process_found(JsFeature::RestSpread, spread.span);
              }
              _ => {}
            }
          }
        }
        // ServiceWorker
        AstKind::MemberExpression(expr) => {
          if let MemberExpression::StaticMemberExpression(static_expr) = expr {
            if let Expression::Identifier(obj) = static_expr.get_first_object() {
              if obj.name == "navigator" {
                if let Some(prop) = expr.static_property_name() {
                  if prop == "serviceWorker" {
                    self.process_found(JsFeature::ServiceWorker, static_expr.span);
                  }
                }
              }
              if obj.name == "performance" {
                if let Some(prop) = expr.static_property_name() {
                  if prop == "now" {
                    self.process_found(JsFeature::PerformanceNow, static_expr.span);
                  }
                }
              }
            }
          }
        }
        // requestIdleCallback
        AstKind::CallExpression(expr) => {
          if expr
            .callee_name()
            .unwrap_or("")
            .contains("requestIdleCallback")
          {
            self.process_found(JsFeature::RequestIdleCallback, expr.span);
          }
        }
        _ => {}
      }
    }

    if !errors.is_empty() {
      println!("Failed to parse JavaScript code");
      for error in errors {
        println!("Error: {}", error);
      }
    }
  }

  pub fn get_features(mut self) -> Vec<FeatureReport> {
    self.found_features = self.features.lock().unwrap().values().cloned().collect();
    self.found_features.clone()
  }

  fn process_found(&self, feature: JsFeature, span: Span) {
    let mut features = self.features.lock().unwrap();
    let browser_support = feature.browser_support();
    self
      .browser_support
      .lock()
      .unwrap()
      .extend(browser_support.clone());
    if let Some(report) = features.get_mut(&feature) {
      report.add_span(span);
      report.support.lock().unwrap().extend(browser_support);
      report.mdn_url = feature.mdn_url();
    } else {
      let mut report = FeatureReport::new(feature, self.browser_support.lock().unwrap().clone());
      report.add_span(span);
      report.support.lock().unwrap().extend(browser_support);
      report.mdn_url = feature.mdn_url();
      features.insert(feature, report);
    }
  }
}
