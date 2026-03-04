mod core;
mod games;
mod interface;
mod joueur;
mod partie;
mod utils;
mod communication;
use communication::MessageClient;
use tokio::net::TcpListener;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use serde_json;

use crate::joueur::Joueur;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let mut joueurs_connectes = Vec::new();

    println!("En attente de 2 joueurs...");

    while joueurs_connectes.len() < 2 {
        let (mut socket, addr) = listener.accept().await?;

        let mut tampon = [0; 1024];
        let n = socket.read(&mut tampon).await?;

        let msg: MessageClient = serde_json::from_slice(&tampon[..n])
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Format invalide"))?;

        if let MessageClient::Connexion { pseudo } = msg {
            println!("{} vient de rejoindre la table ({})", pseudo, addr);
            let nouveau_joueur = Joueur::nouveau(pseudo, 1000);
            joueurs_connectes.push((nouveau_joueur, socket));
        }
    }

    println!("Tous les joueurs sont là, la partie commence !");
    Ok(())
}
