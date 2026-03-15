#[tokio::main]
async fn main() -> std::io::Result<()> {
    poker_rust::network::server::run_poker_server("0.0.0.0:8080").await
}
