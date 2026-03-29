use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::{FromRow, PgPool, Row};

#[derive(FromRow)]
pub struct Joueur {
    pub id: i32,
    pub pseudo: String,
    pub jetons: i32,
}

pub async fn inscrire(pool: &PgPool, pseudo: &str, mot_de_passe: &str) -> Result<Joueur, String> {
    let hash_mdp = hash(mot_de_passe, DEFAULT_COST).map_err(|e| e.to_string())?;

    let joueur = sqlx::query_as::<_, Joueur>(
        "INSERT INTO joueurs (pseudo, password) VALUES ($1, $2) RETURNING id, pseudo, jetons",
    )
    .bind(pseudo)
    .bind(hash_mdp)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
            format!("Le pseudo '{}' est déjà pris.", pseudo)
        }
        _ => e.to_string(),
    })?;

    Ok(joueur)
}

pub async fn authentifier(
    pool: &PgPool,
    pseudo: &str,
    mot_de_passe: &str,
) -> Result<Option<Joueur>, String> {
    let row = sqlx::query(
        "SELECT id, pseudo, password, jetons FROM joueurs WHERE pseudo = $1",
    )
    .bind(pseudo)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    match row {
        None => Ok(None),
        Some(r) => {
            let password: String = r.try_get("password").map_err(|e| e.to_string())?;
            let valide = verify(mot_de_passe, &password).map_err(|e| e.to_string())?;
            if valide {
                Ok(Some(Joueur {
                    id: r.try_get("id").map_err(|e| e.to_string())?,
                    pseudo: r.try_get("pseudo").map_err(|e| e.to_string())?,
                    jetons: r.try_get("jetons").map_err(|e| e.to_string())?,
                }))
            } else {
                Ok(None)
            }
        }
    }
}

pub async fn maj_jetons(pool: &PgPool, id: i32, jetons: i32) -> Result<(), String> {
    sqlx::query("UPDATE joueurs SET jetons = $1 WHERE id = $2")
        .bind(jetons)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
