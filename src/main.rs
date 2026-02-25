mod carte;
mod interface;
mod joueur;
mod partie;
mod utils;
mod communication;
use partie::Partie;
use utils::{demander, demander_u32};
use communication::MessageClient;
use communication::MessageServeur;
use tokio::net::TcpListener;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use serde_json;

use crate::joueur::Joueur;

// fn main() {
//     println!("=== Poker CLI (Texas Hold'em) ===");

//     let nb_joueurs = demander_u32("Nombre de joueurs (2-10): ", 2, 10) as usize;
//     let mut noms = Vec::with_capacity(nb_joueurs);
//     for i in 1..=nb_joueurs {
//         let nom = demander(&format!("Nom du joueur {}: ", i))
//             .ok()
//             .filter(|s| !s.is_empty())
//             .unwrap_or_else(|| format!("Joueur{}", i));
//         noms.push(nom);
//     }

//     let jetons_depart = demander_u32("Jetons de depart par joueur: ", 10, 10_000);
//     let small_blind = demander_u32("Small blind: ", 1, jetons_depart);
//     let big_blind = demander_u32("Big blind: ", small_blind + 1, jetons_depart);

//     let mut partie = Partie::nouvelle(noms, jetons_depart, small_blind, big_blind);
//     partie.jouer_session_cli();
// }


#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let mut joueurs_connectes = Vec::new();

    println!("En attente de 2 joueurs...");

    while joueurs_connectes.len() < 2 {
        let (mut socket, addr) = listener.accept().await?;
        
        //Lire le pseudo
        let mut tampon = [0; 1024];
        let n = socket.read(&mut tampon).await?;
        
        // Désérialiser le message JSON
        let msg: MessageClient = serde_json::from_slice(&tampon[..n])
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Format invalide"))?;

        if let MessageClient::Connexion { pseudo } = msg {
            println!("{} vient de rejoindre la table ({})", pseudo, addr);
            
            // Créer l'objet Joueur
            let nouveau_joueur = Joueur::nouveau(pseudo, 1000); //Modifier plus tard les jetons par défaut
            joueurs_connectes.push((nouveau_joueur, socket));
        }
    }

    println!("Tous les joueurs sont là, la partie commence !");
    Ok(())
}