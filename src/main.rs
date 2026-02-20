mod carte;
mod interface;
mod joueur;
mod partie;
mod utils;

use partie::Partie;
use utils::{demander, demander_u32};

fn main() {
    println!("=== Poker CLI (Texas Hold'em) ===");

    let nb_joueurs = demander_u32("Nombre de joueurs (2-10): ", 2, 10) as usize;
    let mut noms = Vec::with_capacity(nb_joueurs);
    for i in 1..=nb_joueurs {
        let nom = demander(&format!("Nom du joueur {}: ", i))
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Joueur{}", i));
        noms.push(nom);
    }

    let jetons_depart = demander_u32("Jetons de depart par joueur: ", 10, 10_000);
    let small_blind = demander_u32("Small blind: ", 1, jetons_depart);
    let big_blind = demander_u32("Big blind: ", small_blind + 1, jetons_depart);

    let mut partie = Partie::nouvelle(noms, jetons_depart, small_blind, big_blind);
    partie.jouer_session_cli();
}
