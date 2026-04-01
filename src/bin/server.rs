#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://poker:poker@localhost:5432/poker".to_string()
    });

    let pool = match sqlx::PgPool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Impossible de se connecter a PostgreSQL.");
            eprintln!("DATABASE_URL utilisee: {database_url}");
            eprintln!("Erreur: {err}");
            eprintln!(
                "Configure un .env avec DATABASE_URL ou demarre PostgreSQL sur cette URL."
            );
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("postgres inaccessible: {err}"),
            ));
        }
    };

    poker_rust::network::server::run_poker_server("0.0.0.0:9090", pool).await
}
