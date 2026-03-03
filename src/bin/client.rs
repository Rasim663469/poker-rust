use poker_rust::communication::{MessageClient, MessageServeur, ActionJoueur};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

fn main() -> eframe::Result<()> {

    print!("Entre ton pseudo : ");
    let _ = io::stdout().flush();
    let mut pseudo = String::new();
    let _ = io::stdin().read_line(&mut pseudo);
    let pseudo = pseudo.trim().to_string();
    let pseudo = if pseudo.is_empty() { "JoueurMystere".to_string() } else { pseudo };

    let (tx_reseau_vers_gui, rx_reseau_vers_gui) = mpsc::sync_channel::<MessageServeur>(64);
    let (tx_gui_vers_reseau, rx_gui_vers_reseau) = mpsc::channel::<ActionJoueur>();
    let (tx_pret, rx_pret) = mpsc::sync_channel::<()>(1);

    thread::spawn(move || {
        let _ = rx_pret.recv_timeout(std::time::Duration::from_secs(10));

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut stream = match TcpStream::connect("127.0.0.1:8080").await {
                Ok(s) => {
                    println!("Connecté au serveur !");
                    s
                }
                Err(e) => {
                    eprintln!("Serveur introuvable : {}", e);
                    let _ = tx_reseau_vers_gui.send(MessageServeur::Bienvenue {
                        message: "Impossible de se connecter au serveur (127.0.0.1:8080)".to_string()
                    });
                    return;
                }
            };

            let ident = MessageClient::Connexion { pseudo };
            let bytes = serde_json::to_vec(&ident).unwrap();
            if let Err(e) = stream.write_all(&bytes).await {
                eprintln!("Erreur envoi connexion : {}", e);
                return;
            }

            let mut buffer = vec![0u8; 8192];

            loop {
                let n = match stream.read(&mut buffer).await {
                    Ok(0) => {
                        let _ = tx_reseau_vers_gui.send(MessageServeur::Bienvenue {
                            message: "[ Connexion fermée par le serveur. ]".to_string()
                        });
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        let _ = tx_reseau_vers_gui.send(MessageServeur::Bienvenue {
                            message: format!("[ Erreur réseau : {} ]", e)
                        });
                        break;
                    }
                };

                let deserializer = serde_json::Deserializer::from_slice(&buffer[..n]);
                let mut quitter = false;
                for msg_result in deserializer.into_iter::<MessageServeur>() {
                    match msg_result {
                        Ok(msg) => {
                            let is_action = matches!(msg, MessageServeur::DemanderAction { .. } | MessageServeur::DemanderConfiguration);

                            if tx_reseau_vers_gui.send(msg).is_err() {
                                quitter = true;
                                break;
                            }

                            if is_action {
                                match rx_gui_vers_reseau.recv() {
                                    Ok(action) => {
                                        let reponse = MessageClient::Action(action);
                                        let bytes_reponse = serde_json::to_vec(&reponse).unwrap();
                                        if let Err(_) = stream.write_all(&bytes_reponse).await {
                                            quitter = true;
                                            break;
                                        }
                                    }
                                    Err(_) => {
                                        quitter = true;
                                        break;
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
                if quitter { break; }
            }
        });
    });


    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 820.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Poker Rust - Client",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let mut app = poker_rust::interface::gui::CasinoApp::default();
            app.rx_reseau = Some(rx_reseau_vers_gui);
            app.tx_reseau = Some(tx_gui_vers_reseau);
            let _ = tx_pret.send(());
            Ok(Box::new(app))
        }),
    )
}