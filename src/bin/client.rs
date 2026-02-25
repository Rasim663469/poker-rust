use poker_rust::communication::{MessageClient, MessageServeur, ActionJoueur};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("Connecté au serveur de Poker !");

    print!("Entre ton pseudo : ");
    io::stdout().flush()?;
    let mut pseudo = String::new();
    io::stdin().read_line(&mut pseudo)?;
    
    let ident = MessageClient::Connexion { pseudo: pseudo.trim().to_string() };
    let bytes = serde_json::to_vec(&ident).unwrap();
    stream.write_all(&bytes).await?;

    let mut buffer = [0; 2048];
    loop {
        let n = stream.read(&mut buffer).await?;
        if n == 0 { 
            println!("Déconnecté du serveur.");
            break; 
        }

        let mut cursor = std::io::Cursor::new(&buffer[..n]);
        let deserializer = serde_json::Deserializer::from_reader(cursor);
        let iter = deserializer.into_iter::<MessageServeur>();
        for msg_result in iter {
            if let Ok(msg) = msg_result {
                match msg {
                    MessageServeur::Bienvenue { message } => println!("\n[INFO] {}", message),
                    
                    MessageServeur::MesCartes { cartes } => {
                        println!("\n--- TES CARTES ---");
                        for c in cartes { println!("  [{}]", c); } 
                    },

                    MessageServeur::MajTable { pot, cartes_communes } => {
                        let cartes_formatees = cartes_communes.iter()
                        .map(|c| format!("[{}]", c)) // Utilise le Display : AC, 10T, etc.
                        .collect::<Vec<_>>()
                        .join(" ");
                        println!("\n--- TABLE --- Pot: {} | Cartes: {}", pot, cartes_formatees);
                    },

                    MessageServeur::DemanderAction { to_call, peut_relancer ,jetons_restants} => {
                        let action = gerer_mon_tour(to_call, peut_relancer,jetons_restants);
                        
                        let reponse = MessageClient::Action(action);
                        let bytes_reponse = serde_json::to_vec(&reponse).unwrap();
                        stream.write_all(&bytes_reponse).await?;
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn gerer_mon_tour(to_call: u32, peut_relancer: bool,jetons: u32) -> ActionJoueur {
    loop {
        println!("\n--- À TON TOUR (Jetons : {}) ---", jetons);
        if to_call == 0 {
            print!("\nC'est ton tour ! [c=check, r=relancer, f=fold] : ");
        } else {
            print!("\nC'est ton tour ! (A payer: {}) [s=suivre, r=relancer, f=fold] : ", to_call);
        }
        io::stdout().flush().unwrap();

        let mut entree = String::new();
        io::stdin().read_line(&mut entree).unwrap();
        
        match entree.trim().to_lowercase().as_str() {
            "f" => return ActionJoueur::Fold,
            "c" if to_call == 0 => return ActionJoueur::Call,
            "s" if to_call > 0 => return ActionJoueur::Call,
            "r" if peut_relancer => {
                print!("Montant total de ta mise : ");
                io::stdout().flush().unwrap();
                let mut montant_str = String::new();
                io::stdin().read_line(&mut montant_str).unwrap();
                if let Ok(m) = montant_str.trim().parse::<u32>() {
                    return ActionJoueur::Raise(m);
                }
                println!("Montant invalide.");
            },
            _ => println!("Action non reconnue ou impossible."),
        }
    }
}