#[tokio::main]
async fn main() -> std::io::Result<()> {
    //127.0.0.1 pour etre en local
    //0.0.0.0 pour etre accessible depuis l'exterieur
    poker_rust::network::server::run_poker_server("0.0.0.0:8080").await
}
