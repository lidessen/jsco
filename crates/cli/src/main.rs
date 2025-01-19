use jsco_cli::run;
use std::env;

#[tokio::main]
async fn main() {
  run(env::args().collect()).await;
}
