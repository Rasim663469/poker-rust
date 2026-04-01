#[tokio::main]
async fn main() -> std::io::Result<()> {
    poker_rust::network::client::run_poker_client("162.38.111.42:9090").await
}
