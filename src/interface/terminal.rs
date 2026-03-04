use crate::core::utils::{demander, demander_u32};
use crate::games::blackjack::engine::{EtatBlackjack, JeuBlackjack};
use crate::games::poker::engine::Partie;

pub fn lancer_casino_cli() {
    println!("=== Casino CLI ===");
    println!("1. Poker Texas Hold'em");
    println!("2. Blackjack");
    let choix = demander_u32("Choisis un jeu (1-2): ", 1, 2);
    if choix == 1 {
        lancer_poker_cli();
    } else {
        lancer_blackjack_cli();
    }
}

pub fn lancer_poker_cli() {
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

pub fn lancer_blackjack_cli() {
    println!("=== Blackjack CLI ===");
    let nb_joueurs = demander_u32("Nombre de joueurs total (2-6, toi inclus): ", 2, 6) as usize;
    let jetons_depart = demander_u32("Jetons de depart: ", 10, 50_000);
    let mut jeu = JeuBlackjack::nouveau(nb_joueurs, jetons_depart);

    loop {
        if jeu.jetons_humain() == 0 {
            println!("Tu n'as plus de jetons. Fin de session.");
            break;
        }

        let mise_max = jeu.jetons_humain();
        let mise = demander_u32(
            &format!("Mise (1..={} , 0 pour quitter): ", mise_max),
            0,
            mise_max,
        );
        if mise == 0 {
            break;
        }

        if let Err(err) = jeu.commencer_manche(mise) {
            println!("{}", err);
            continue;
        }

        while jeu.etat == EtatBlackjack::TourJoueur || jeu.etat == EtatBlackjack::TourCroupier {
            jeu.avancer_automatique();
            if jeu.etat != EtatBlackjack::TourJoueur {
                break;
            }
            if !jeu.est_tour_humain() {
                continue;
            }
            afficher_etat_blackjack(&jeu);
            let action = demander("Action [h=hit, s=stand]: ")
                .ok()
                .unwrap_or_default()
                .to_lowercase();
            match action.as_str() {
                "h" => jeu.joueur_hit(),
                "s" => jeu.joueur_stand(),
                _ => println!("Action invalide."),
            }
        }

        afficher_etat_blackjack(&jeu);
        println!("{}", jeu.message);
        println!("Jetons restants: {}\n", jeu.jetons_humain());
    }
}

fn afficher_etat_blackjack(jeu: &JeuBlackjack) {
    let main_joueur = jeu.joueurs[0]
        .main
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let main_croupier = if jeu.croupier_cachee() {
        if jeu.main_croupier.len() >= 2 {
            format!("?? {}", jeu.main_croupier[1])
        } else {
            "??".to_string()
        }
    } else {
        jeu.main_croupier
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };

    println!("Croupier: {} (score: {})", main_croupier, jeu.score_croupier_visible());
    println!("Joueur:   {} (score: {})", main_joueur, jeu.score_joueur(0));
    for i in 1..jeu.joueurs.len() {
        let j = &jeu.joueurs[i];
        if !j.actif() && j.jetons == 0 {
            continue;
        }
        let main = if j.main.is_empty() {
            "-".to_string()
        } else {
            j.main.iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!("{}: {} (score: {}, mise: {})", j.nom, main, jeu.score_joueur(i), j.mise);
    }
}
