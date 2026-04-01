#[tokio::main]
async fn main() -> std::io::Result<()> {
    poker_rust::network::client::run_poker_client("127.0.0.1:9090").await
}
