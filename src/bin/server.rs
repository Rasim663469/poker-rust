use poker_rust::partie::Partie;
use poker_rust::communication::{MessageClient, MessageServeur,ActionJoueur};
use tokio::net::TcpListener;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let mut flux_joueurs = Vec::new();

    println!("En attente du créateur de la partie (Hôte)...");

    let (mut socket_hote, _addr_hote) = listener.accept().await?;
    let mut tampon = [0; 1024];
    let n = socket_hote.read(&mut tampon).await?;
    let msg: MessageClient = serde_json::from_slice(&tampon[..n]).unwrap();

    let pseudo_hote = if let MessageClient::Connexion { pseudo } = msg { pseudo } else { "Hôte".to_string() };
    println!("L'hôte {} s'est connecté. Attente de sa configuration...", pseudo_hote);
    
    let demande = MessageServeur::DemanderConfiguration;
    socket_hote.write_all(&serde_json::to_vec(&demande).unwrap()).await?;

    let n = socket_hote.read(&mut tampon).await?;
    let reponse: MessageClient = serde_json::from_slice(&tampon[..n]).unwrap();
    
    let (nb_joueurs_attendus, jetons_depart) = if let MessageClient::Action(ActionJoueur::ConfigurerPartie { nb_joueurs, jetons }) = reponse {
        (nb_joueurs as usize, jetons)
    } else {
        (2, 1000) 
    };

    println!("Configuration reçue : {} joueurs, {} jetons chacun.", nb_joueurs_attendus, jetons_depart);
    println!("En attente de {} joueurs supplémentaires...", nb_joueurs_attendus - 1);

    flux_joueurs.push((pseudo_hote, socket_hote));

    while flux_joueurs.len() < nb_joueurs_attendus {
        let (mut socket, addr) = listener.accept().await?;

        let mut tampon = [0; 1024];
        let n = socket.read(&mut tampon).await?;

        let msg: MessageClient = serde_json::from_slice(&tampon[..n])
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Format invalide"))?;

        if let MessageClient::Connexion { pseudo } = msg {
            println!("{} a rejoint la table ({})", pseudo, addr);

            let bienvenue = MessageServeur::Bienvenue {
                message: format!("Salut {}, attendons les autres joueurs...", pseudo)
            };
            socket.write_all(&serde_json::to_vec(&bienvenue).unwrap()).await?;

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