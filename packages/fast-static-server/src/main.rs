use std::path::PathBuf;

use clap::Parser;
use fast_static_server::build_app;

/// A Rust port of the npm `serve` package: serve a directory over HTTP.
#[derive(Parser)]
struct Cli {
    /// Directory to serve
    #[arg(default_value = ".")]
    dir: PathBuf,

    /// Port to listen on
    #[arg(short = 'l', long = "listen", default_value_t = 3000)]
    port: u16,

    /// Host/interface to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Single Page App mode: rewrite all not-found requests to index.html
    #[arg(short = 's', long)]
    single: bool,

    /// Send permissive CORS headers (Access-Control-Allow-Origin: *)
    #[arg(long)]
    cors: bool,

    /// Show debug info (log every request)
    #[arg(short = 'd', long)]
    debug: bool,

    /// Send `Cache-Control: public, max-age=<seconds>` for served files (off by default)
    #[arg(short = 'c', long = "cache-control", value_name = "SECONDS")]
    cache_control: Option<u64>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let root = cli
        .dir
        .canonicalize()
        .unwrap_or_else(|e| panic!("cannot access directory {:?}: {e}", cli.dir));

    let app = build_app(
        root.clone(),
        cli.single,
        cli.cors,
        cli.debug,
        cli.cache_control,
    );

    let addr = format!("{}:{}", cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {addr}: {e}"));

    println!("Serving {}", root.display());
    println!("  Local: http://localhost:{}", cli.port);

    axum::serve(listener, app).await.unwrap();
}
