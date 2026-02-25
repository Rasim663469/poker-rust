use poker_rust::partie::Partie;
use poker_rust::joueur::Joueur;
use poker_rust::utils::{demander, demander_u32};
use poker_rust::communication::{MessageClient, MessageServeur}; 
use tokio::net::TcpListener;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use poker_rust::carte::Paquet;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let nb_joueurs_attendus = demander_u32("Nombre de joueurs pour cette partie (2-10) : ", 2, 10) as usize;
    let jetons_depart = demander_u32("Jetons de départ : ", 100, 10000);

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let mut flux_joueurs = Vec::new();



    println!("En attente de {} joueurs...",nb_joueurs_attendus);

    while flux_joueurs.len() < nb_joueurs_attendus {
        let (mut socket, addr) = listener.accept().await?;
        
        let mut tampon = [0; 1024];
        let n = socket.read(&mut tampon).await?;
        
        let msg: MessageClient = serde_json::from_slice(&tampon[..n])
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Format invalide"))?;

        if let MessageClient::Connexion { pseudo } = msg {
            println!("{} a rejoint la table ({})", pseudo, addr);
            flux_joueurs.push((pseudo, socket));
        }
    }


    let mut noms = Vec::new();
    let mut sockets = Vec::new();
    for (pseudo, socket) in flux_joueurs {
        noms.push(pseudo);
        sockets.push(socket);
    }

  
    let mut partie = Partie::nouvelle(noms, jetons_depart, 10, 20, sockets);

    println!("Tous les joueurs sont là, la partie commence !");

   
    partie.jouer_session_cli().await?;

    Ok(())
}