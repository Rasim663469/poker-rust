use crate::network::protocol::{ActionJoueur, MessageClient, MessageServeur};
use crate::network::{recv_json, send_json};
use std::io::{self, Write};
use tokio::net::TcpStream;

pub async fn run_poker_client(addr: &str) -> io::Result<()> {
    let mut stream = TcpStream::connect(addr).await?;
    let pseudo = demander_str("Pseudo: ");
    send_json(
        &mut stream,
        &MessageClient::Connexion {
            pseudo: if pseudo.is_empty() { "Joueur".to_string() } else { pseudo },
        },
    )
    .await?;

    loop {
        let msg: MessageServeur = match recv_json(&mut stream).await {
            Ok(m) => m,
            Err(_) => {
                println!("Connexion fermee par le serveur.");
                return Ok(());
            }
        };

        match msg {
            MessageServeur::Bienvenue { message } => println!("[INFO] {message}"),
            MessageServeur::MesCartes { cartes } => {
                let main = cartes
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("Tes cartes: {main}");
            }
            MessageServeur::MajTable { pot, cartes_communes } => {
                let board = cartes_communes
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("Table -> Pot: {pot} | Board: {board}");
            }
            MessageServeur::DemanderConfiguration => {
                let nb = demander_u32("Nombre de joueurs (2-6): ", 2, 6);
                let jetons = demander_u32("Jetons de depart (>=50): ", 50, 100_000);
                send_json(
                    &mut stream,
                    &MessageClient::Action(ActionJoueur::ConfigurerPartie {
                        nb_joueurs: nb,
                        jetons,
                    }),
                )
                .await?;
            }
            MessageServeur::DemanderAction {
                to_call,
                peut_relancer,
                jetons_restants,
            } => {
                println!(
                    "Ton tour -> a payer: {to_call} | jetons: {jetons_restants} | relance: {}",
                    if peut_relancer { "oui" } else { "non" }
                );

                let action = if to_call == 0 {
                    demander_str("Action [c=check, r=raise, f=fold]: ")
                } else {
                    demander_str("Action [s=call, r=raise, f=fold]: ")
                };

                let action = match action.trim().to_lowercase().as_str() {
                    "f" => ActionJoueur::Fold,
                    "c" if to_call == 0 => ActionJoueur::Check,
                    "s" if to_call > 0 => ActionJoueur::Call,
                    "r" if peut_relancer => {
                        let min = if to_call == 0 { 20 } else { to_call + 20 };
                        let max = jetons_restants + to_call;
                        let total = demander_u32(
                            &format!("Montant total de mise ({min}..={max}): "),
                            min,
                            max,
                        );
                        ActionJoueur::Raise(total)
                    }
                    _ => {
                        if to_call == 0 {
                            ActionJoueur::Check
                        } else {
                            ActionJoueur::Call
                        }
                    }
                };

                send_json(&mut stream, &MessageClient::Action(action)).await?;
            }
            MessageServeur::AnnonceAction { nom, action } => {
                println!("{nom}: {action}");
            }
            MessageServeur::Erreur { message } => {
                println!("[ERREUR] {message}");
            }
        }
    }
}

fn demander_str(message: &str) -> String {
    print!("{message}");
    let _ = io::stdout().flush();
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
    s.trim().to_string()
}

fn demander_u32(message: &str, min: u32, max: u32) -> u32 {
    loop {
        let s = demander_str(message);
        match s.parse::<u32>() {
            Ok(v) if v >= min && v <= max => return v,
            _ => println!("Entree invalide ({min}..={max})"),
        }
    }
}
