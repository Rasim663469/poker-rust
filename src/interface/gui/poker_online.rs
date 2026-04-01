use crate::network::protocol::{ActionJoueur, MessageClient, MessageServeur};
use crate::network::{recv_json, send_json};
use eframe::egui;
use super::draw::{dessiner_carte, dessiner_joueur_zone};
use super::theme::{back_button, panel_frame, premium_button, section_title, status_panel, subpanel_frame, TABLE_GREEN, TEXT_DIM, GOLD_SOFT};
use std::sync::mpsc;
use std::thread;
use tokio::net::TcpStream;

pub(super) struct OnlinePokerState {
    pub(super) adresse: String,
    pub(super) est_hote: bool,
    pub(super) nb_joueurs: u32,
    pub(super) jetons_depart: u32,
    pub(super) connecte: bool,
    pub(super) statut: String,
    pub(super) logs: Vec<String>,
    pub(super) main: Vec<crate::core::cards::Carte>,
    pub(super) board: Vec<crate::core::cards::Carte>,
    pub(super) pot: u32,
    pub(super) en_attente_action: bool,
    pub(super) to_call: u32,
    pub(super) peut_relancer: bool,
    pub(super) jetons_restants: u32,
    pub(super) raise_total_input: u32,
}

impl Default for OnlinePokerState {
    fn default() -> Self {
        Self {
            adresse: "127.0.0.1:9090".to_string(),
            est_hote: false,
            nb_joueurs: 2,
            jetons_depart: 1000,
            connecte: false,
            statut: "Non connecte".to_string(),
            logs: Vec::new(),
            main: Vec::new(),
            board: Vec::new(),
            pot: 0,
            en_attente_action: false,
            to_call: 0,
            peut_relancer: false,
            jetons_restants: 0,
            raise_total_input: 20,
        }
    }
}

impl super::CasinoApp {
    pub(super) fn pomper_messages_online(&mut self) {
        if let Some(rx) = &self.rx_online {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    MessageServeur::Bienvenue { message } => {
                        self.poker_online.statut = message.clone();
                        self.poker_online.logs.push(format!("[INFO] {message}"));
                    }
                    MessageServeur::MesCartes { cartes } => {
                        self.poker_online.main = cartes;
                        self.poker_online.logs.push("--- Nouvelle main ---".to_string());
                    }
                    MessageServeur::MajTable { pot, cartes_communes } => {
                        self.poker_online.pot = pot;
                        self.poker_online.board = cartes_communes;
                    }
                    MessageServeur::DemanderAction {
                        to_call,
                        peut_relancer,
                        jetons_restants,
                    } => {
                        self.poker_online.en_attente_action = true;
                        self.poker_online.to_call = to_call;
                        self.poker_online.peut_relancer = peut_relancer;
                        self.poker_online.jetons_restants = jetons_restants;
                        self.poker_online.raise_total_input = (to_call + 20).max(20);
                        self.poker_online.statut = "Ton tour".to_string();
                    }
                    MessageServeur::AnnonceAction { nom, action } => {
                        self.poker_online.logs.push(format!("{nom}: {action}"));
                    }
                    MessageServeur::DemanderConfiguration => {
                        self.poker_online.logs.push(
                            "Le serveur demande la configuration (envoyee automatiquement si tu es hote)."
                                .to_string(),
                        );
                    }
                    MessageServeur::Erreur { message } => {
                        self.poker_online.logs.push(format!("[ERREUR] {message}"));
                        self.poker_online.statut = message;
                        self.poker_online.connecte = false;
                    }
                    MessageServeur::AuthOk { jetons } => {
                        self.poker_online.logs.push(format!("[AUTH] Connecté. Jetons: {jetons}"));
                        self.poker_online.jetons_restants = jetons;
                    }
                    MessageServeur::AuthEchec { raison } => {
                        self.poker_online.logs.push(format!("[AUTH ECHEC] {raison}"));
                        self.poker_online.statut = raison;
                        self.poker_online.connecte = false;
                    }
                }
            }
        }
    }

    pub(super) fn ui_poker_online(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if back_button(ui, "< Retour choix mode").clicked() {
                self.poker_vue = super::PokerVue::Choix;
            }
            ui.separator();
            ui.label("Multijoueur TCP");
        });

        if !self.poker_online.connecte {
            panel_frame().show(ui, |ui| {
                section_title(ui, "Connexion online", "Rejoins une table distante ou ouvre une session hôte.");
                ui.add_space(10.0);
                subpanel_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Adresse:");
                        ui.text_edit_singleline(&mut self.poker_online.adresse);
                    });
                    ui.label(format!("Session: {}", self.joueur_pseudo));
                    ui.label(
                        egui::RichText::new(format!(
                            "Capital disponible: {} jetons",
                            self.banque_joueur
                        ))
                        .color(TEXT_DIM),
                    );
                    ui.checkbox(&mut self.poker_online.est_hote, "Je suis l'hote");
                    if self.poker_online.est_hote {
                        ui.add(
                            egui::Slider::new(&mut self.poker_online.nb_joueurs, 2..=6)
                                .text("Nombre de joueurs"),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Le solde des joueurs vient de leur compte, pas d'un capital fixe de table."
                            )
                            .color(TEXT_DIM),
                        );
                    }
                });

                if premium_button(ui, "Se connecter").clicked()
                {
                    self.demarrer_client_online();
                }
                ui.label(egui::RichText::new(format!("Statut: {}", self.poker_online.statut)).color(TEXT_DIM));
            });
            return;
        }

        ui.horizontal(|ui| {
            if back_button(ui, "Se deconnecter").clicked() {
                self.arreter_client_online();
            }
            ui.label(format!("Statut: {}", self.poker_online.statut));
        });

        ui.separator();
        let table_height = 430.0;
        let table_width = (ui.available_width() - 12.0).max(760.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(table_width, table_height), egui::Sense::hover());
        dessiner_table_online(ui, rect, &self.poker_online);

        ui.add_space(8.0);
        status_panel(
            ui,
            format!(
                "Pot: {} | Jetons: {}",
                self.poker_online.pot, self.poker_online.jetons_restants
            ),
        );

        if self.poker_online.en_attente_action {
            ui.separator();
            ui.label(
                egui::RichText::new(format!("Ton tour. A payer: {}", self.poker_online.to_call))
                    .color(GOLD_SOFT),
            );
            ui.horizontal(|ui| {
                if ui
                    .button(if self.poker_online.to_call == 0 {
                        "Check"
                    } else {
                        "Call"
                    })
                    .clicked()
                {
                    let action = if self.poker_online.to_call == 0 {
                        ActionJoueur::Check
                    } else {
                        ActionJoueur::Call
                    };
                    self.envoyer_action_online(action);
                }
                if ui.button("Fold").clicked() {
                    self.envoyer_action_online(ActionJoueur::Fold);
                }
            });

            if self.poker_online.peut_relancer {
                let min = (self.poker_online.to_call + 20).max(20);
                let max = self.poker_online.jetons_restants + self.poker_online.to_call;
                if self.poker_online.raise_total_input < min {
                    self.poker_online.raise_total_input = min;
                }
                if self.poker_online.raise_total_input > max {
                    self.poker_online.raise_total_input = max;
                }
                ui.add(
                    egui::Slider::new(&mut self.poker_online.raise_total_input, min..=max)
                        .text("Montant total"),
                );
                if ui.button("Raise").clicked() {
                    self.envoyer_action_online(ActionJoueur::Raise(
                        self.poker_online.raise_total_input,
                    ));
                }
            }
        }

        ui.separator();
        ui.label("Historique:");
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for l in self.poker_online.logs.iter().rev().take(60).rev() {
                ui.label(l);
            }
        });
    }

    pub(super) fn demarrer_client_online(&mut self) {
        if self.poker_online.connecte {
            return;
        }

        let adresse = self.poker_online.adresse.trim().to_string();
        let pseudo = self.joueur_pseudo.trim().to_string();
        let db_id = self.joueur_db_id;
        let est_hote = self.poker_online.est_hote;
        let nb_joueurs = self.poker_online.nb_joueurs;
        let jetons_depart = self.poker_online.jetons_depart;

        if adresse.is_empty() {
            self.poker_online.statut = "Adresse vide".to_string();
            return;
        }
        if pseudo.is_empty() {
            self.poker_online.statut = "Session invalide: pseudo vide".to_string();
            return;
        }
        let Some(db_id) = db_id else {
            self.poker_online.statut = "Session invalide: reconnecte-toi".to_string();
            return;
        };

        let (tx_srv_to_ui, rx_srv_to_ui) = mpsc::channel::<MessageServeur>();
        let (tx_ui_to_srv, rx_ui_to_srv) = mpsc::channel::<ActionJoueur>();

        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx_srv_to_ui.send(MessageServeur::Erreur {
                        message: format!("Runtime tokio impossible: {e}"),
                    });
                    return;
                }
            };

            rt.block_on(async move {
                let mut stream = match TcpStream::connect(&adresse).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx_srv_to_ui.send(MessageServeur::Erreur {
                            message: format!("Connexion impossible vers {adresse}: {e}"),
                        });
                        return;
                    }
                };

                let auth_msg = MessageClient::Session {
                    db_id,
                    pseudo,
                };

                if send_json(&mut stream, &auth_msg).await.is_err() {
                    let _ = tx_srv_to_ui.send(MessageServeur::Erreur {
                        message: "Echec envoi du message d'authentification.".to_string(),
                    });
                    return;
                }

                match recv_json::<MessageServeur, _>(&mut stream).await {
                    Ok(MessageServeur::AuthOk { jetons }) => {
                        let _ = tx_srv_to_ui.send(MessageServeur::AuthOk { jetons });
                    }
                    Ok(MessageServeur::AuthEchec { raison }) => {
                        let _ = tx_srv_to_ui.send(MessageServeur::AuthEchec { raison });
                        return;
                    }
                    _ => {
                        let _ = tx_srv_to_ui.send(MessageServeur::Erreur {
                            message: "Réponse inattendue lors de l'auth.".to_string(),
                        });
                        return;
                    }
                }
                loop {
                    let msg: MessageServeur = match recv_json(&mut stream).await {
                        Ok(m) => m,
                        Err(_) => {
                            let _ = tx_srv_to_ui.send(MessageServeur::Erreur {
                                message: "Connexion fermee.".to_string(),
                            });
                            break;
                        }
                    };

                    if let MessageServeur::DemanderConfiguration = msg {
                        if est_hote {
                            let _ = send_json(
                                &mut stream,
                                &MessageClient::Action(ActionJoueur::ConfigurerPartie {
                                    nb_joueurs,
                                    jetons: jetons_depart,
                                }),
                            )
                            .await;
                        } else {
                            let _ = send_json(
                                &mut stream,
                                &MessageClient::Action(ActionJoueur::ConfigurerPartie {
                                    nb_joueurs: 2,
                                    jetons: 1000,
                                }),
                            )
                            .await;
                        }
                    }

                    let demande_action = matches!(msg, MessageServeur::DemanderAction { .. });
                    if tx_srv_to_ui.send(msg).is_err() {
                        break;
                    }

                    if demande_action {
                        let action = rx_ui_to_srv.recv().unwrap_or(ActionJoueur::Fold);
                        let _ = send_json(&mut stream, &MessageClient::Action(action)).await;
                    }
                }
            });
        });

        self.rx_online = Some(rx_srv_to_ui);
        self.tx_online = Some(tx_ui_to_srv);
        self.poker_online.connecte = true;
        self.poker_online.statut = "Connexion en cours...".to_string();
        self.poker_online.logs.clear();
        self.poker_online.main.clear();
        self.poker_online.board.clear();
        self.poker_online.pot = 0;
        self.poker_online.en_attente_action = false;
    }

    pub(super) fn arreter_client_online(&mut self) {
        self.tx_online = None;
        self.rx_online = None;
        self.poker_online.connecte = false;
        self.poker_online.en_attente_action = false;
        self.poker_online.statut = "Deconnecte".to_string();
    }

    pub(super) fn envoyer_action_online(&mut self, action: ActionJoueur) {
        if let Some(tx) = &self.tx_online {
            if tx.send(action).is_ok() {
                self.poker_online.en_attente_action = false;
                self.poker_online.statut = "Action envoyee".to_string();
            } else {
                self.poker_online.statut = "Echec envoi action".to_string();
            }
        }
    }
}

fn dessiner_table_online(ui: &mut egui::Ui, rect: egui::Rect, state: &OnlinePokerState) {
    let painter = ui.painter_at(rect);
    let bg = egui::Color32::from_rgb(13, 30, 24);
    painter.rect_filled(rect, 18.0, bg);

    let table_rect = rect.shrink2(egui::vec2(24.0, 18.0));
    painter.rect_filled(table_rect, 120.0, TABLE_GREEN);
    painter.rect_stroke(
        table_rect,
        120.0,
        egui::Stroke::new(4.0, egui::Color32::from_rgb(132, 85, 50)),
        egui::StrokeKind::Outside,
    );

    let c = table_rect.center();
    let board_origin = egui::pos2(c.x - 150.0, c.y - 36.0);
    for i in 0..5 {
        let x = board_origin.x + i as f32 * 62.0;
        let card_rect =
            egui::Rect::from_min_size(egui::pos2(x, board_origin.y), egui::vec2(54.0, 76.0));
        let card = state.board.get(i);
        dessiner_carte(ui, &painter, card_rect, card, card.is_some());
    }

    let pot_rect = egui::Rect::from_center_size(egui::pos2(c.x, c.y + 62.0), egui::vec2(200.0, 46.0));
    painter.rect_filled(pot_rect, 10.0, egui::Color32::from_rgb(11, 41, 30));
    painter.rect_stroke(
        pot_rect,
        10.0,
        egui::Stroke::new(1.5, egui::Color32::from_rgb(201, 178, 102)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        pot_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("POT  {}", state.pot),
        egui::FontId::proportional(22.0),
        egui::Color32::from_rgb(238, 220, 151),
    );

    dessiner_joueur_zone(
        &painter,
        egui::Rect::from_center_size(
            egui::pos2(c.x, table_rect.top() + 40.0),
            egui::vec2(340.0, 56.0),
        ),
        "Adversaires online",
        0,
        0,
    );
    dessiner_joueur_zone(
        &painter,
        egui::Rect::from_center_size(
            egui::pos2(c.x, table_rect.bottom() - 40.0),
            egui::vec2(340.0, 56.0),
        ),
        if state.en_attente_action { "Toi - ton tour" } else { "Toi" },
        state.jetons_restants,
        state.to_call,
    );

    let opp_cards_y = table_rect.top() + 78.0;
    for i in 0..2 {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 62.0 + i as f32 * 68.0, opp_cards_y),
            egui::vec2(58.0, 84.0),
        );
        dessiner_carte(ui, &painter, card_rect, None, false);
    }

    let hero_cards_y = table_rect.bottom() - 164.0;
    for i in 0..2 {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 62.0 + i as f32 * 68.0, hero_cards_y),
            egui::vec2(58.0, 84.0),
        );
        let card = state.main.get(i);
        dessiner_carte(ui, &painter, card_rect, card, card.is_some());
    }
}
