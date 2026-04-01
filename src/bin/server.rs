#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL doit être défini dans .env");
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Impossible de se connecter à PostgreSQL");
    poker_rust::network::server::run_poker_server("0.0.0.0:8080", pool).await
}

