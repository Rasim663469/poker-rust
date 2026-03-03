use eframe::egui;
use crate::carte::Carte;
use crate::communication::{MessageServeur, ActionJoueur};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EcranCasino {
    Lobby,
    Poker,
}

pub struct TableReseau {
    pub historique: Vec<String>,
    pub mes_cartes: Vec<Carte>,
    pub cartes_communes: Vec<Carte>,
    pub pot: u32,
    pub mes_jetons: u32,
    pub a_mon_tour: bool,
    pub to_call: u32,
    pub peut_relancer: bool,
    pub raise_input: u32,
}

impl Default for TableReseau {
    fn default() -> Self {
        Self {
            historique: vec!["Connecté ! En attente du serveur...".to_string()],
            mes_cartes: Vec::new(),
            cartes_communes: Vec::new(),
            pot: 0,
            mes_jetons: 0,
            a_mon_tour: false,
            to_call: 0,
            peut_relancer: false,
            raise_input: 20,
        }
    }
}

pub struct CasinoApp {
    ecran: EcranCasino,
    pub rx_reseau: Option<std::sync::mpsc::Receiver<MessageServeur>>,
    pub tx_reseau: Option<std::sync::mpsc::Sender<ActionJoueur>>,
    pub table: TableReseau,
    pub nb_joueurs: u32,
    pub jetons_depart: u32,
}

impl Default for CasinoApp {
    fn default() -> Self {
        Self {
            ecran: EcranCasino::Poker,
            rx_reseau: None,
            tx_reseau: None,
            table: TableReseau::default(),
            nb_joueurs: 2,
            jetons_depart: 1000,
        }
    }
}

impl eframe::App for CasinoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.rx_reseau {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    MessageServeur::Bienvenue { message } => {
                        self.table.historique.push(message.trim().to_string());
                    }
                    MessageServeur::MesCartes { cartes } => {
                        self.table.mes_cartes = cartes;
                        self.table.cartes_communes.clear();
                        self.table.historique.push("--- NOUVELLE MAIN ---".to_string());
                    }
                    MessageServeur::MajTable { pot, cartes_communes } => {
                        self.table.pot = pot;
                        self.table.cartes_communes = cartes_communes;
                    }
                    MessageServeur::DemanderAction { to_call, peut_relancer, jetons_restants } => {
                        self.table.a_mon_tour = true;
                        self.table.to_call = to_call;
                        self.table.peut_relancer = peut_relancer;
                        self.table.mes_jetons = jetons_restants;
                        self.table.raise_input = to_call + 20;
                    }
                    MessageServeur::AnnonceAction { nom, action } => {
                        self.table.historique.push(format!("{} -> {}", nom, action));
                    }
                    MessageServeur::DemanderConfiguration => {
                        self.ecran = EcranCasino::Lobby;
                    }
                }
            }
            ctx.request_repaint();
        }

        match self.ecran {
            EcranCasino::Lobby => {
                self.ui_lobby(ctx);
            }
            EcranCasino::Poker => {
                egui::TopBottomPanel::top("casino_header").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Casino Rust - Multijoueur en Ligne");
                    });
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.ui_poker(ui);
                });
            }
        }
    }
}

impl CasinoApp {
    fn ui_poker(&mut self, ui: &mut egui::Ui) {
        let table_width = (ui.available_width() * 0.7).max(400.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(table_width, 500.0), egui::Sense::hover());
                if rect.width() > 50.0 {
                    dessiner_table_reseau(ui, rect, &self.table);
                }

                ui.add_space(20.0);

                if self.table.a_mon_tour {
                    ui.group(|ui| {
                        ui.heading("À TON TOUR !");
                        ui.label(format!("Mes Jetons : {}", self.table.mes_jetons));
                        ui.horizontal(|ui| {
                            if ui.button("Fold (Se Coucher)").clicked() {
                                self.envoyer_action(ActionJoueur::Fold);
                            }

                            let lib_call = if self.table.to_call == 0 { "Check" } else { "Call (Suivre)" };
                            if ui.button(format!("{} {}", lib_call, self.table.to_call)).clicked() {
                                self.envoyer_action(ActionJoueur::Call);
                            }

                            if self.table.peut_relancer {
                                ui.separator();
                                ui.add(egui::Slider::new(&mut self.table.raise_input, self.table.to_call..=self.table.mes_jetons).text("Relance"));
                                if ui.button("Raise (Relancer)").clicked() {
                                    self.envoyer_action(ActionJoueur::Raise(self.table.raise_input));
                                }
                            }
                        });
                    });
                } else {
                    ui.label("Attends que les autres joueurs jouent...");
                }
            });

            ui.separator();
            ui.vertical(|ui| {
                ui.heading("Événements de la partie");
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for msg in &self.table.historique {
                        ui.label(msg);
                    }
                });
            });
        });
    }

    fn envoyer_action(&mut self, action: ActionJoueur) {
        self.table.a_mon_tour = false;
        if let Some(tx) = &self.tx_reseau {
            let _ = tx.send(action);
        }
    }

    fn ui_lobby(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let content_width = 520.0f32;
            let available_width = ui.available_width();
            let padding = ((available_width - content_width) / 2.0).max(0.0);

            let espacement_haut = ui.available_height() / 3.0;
            ui.add_space(espacement_haut);

            ui.horizontal(|ui| {
                ui.add_space(padding);
                ui.vertical(|ui| {
                    ui.set_width(content_width);

                    ui.vertical_centered(|ui| {
                        ui.heading("Création de la partie");
                    });
                    ui.add_space(30.0);

                    egui::Grid::new("lobby_grid")
                        .num_columns(2)
                        .spacing([20.0, 16.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Nombre de joueurs (2-10) :").size(18.0));
                            ui.add(egui::Slider::new(&mut self.nb_joueurs, 2..=10).min_decimals(0));
                            ui.end_row();

                            ui.label(egui::RichText::new("Jetons de départ (100-10000) :").size(18.0));
                            ui.add(egui::Slider::new(&mut self.jetons_depart, 100..=10000).min_decimals(0));
                            ui.end_row();
                        });

                    ui.add_space(40.0);

                    ui.vertical_centered(|ui| {
                        if ui.add_sized([200.0, 50.0], egui::Button::new(egui::RichText::new("Commencer la partie").size(20.0))).clicked() {
                            self.envoyer_action(ActionJoueur::ConfigurerPartie {
                                nb_joueurs: self.nb_joueurs,
                                jetons: self.jetons_depart,
                            });
                            self.ecran = EcranCasino::Poker;
                            self.table.historique.push("Configuration envoyée ! En attente des joueurs...".to_string());
                        }
                    });
                });
            });
        });
    }

}

fn dessiner_table_reseau(ui: &mut egui::Ui, rect: egui::Rect, table: &TableReseau) {
    let painter = ui.painter_at(rect);
    let bg = egui::Color32::from_rgb(13, 30, 24);
    painter.rect_filled(rect, 18.0, bg);

    let table_rect = rect.shrink2(egui::vec2(18.0, 12.0));
    painter.rect_filled(table_rect, 40.0, egui::Color32::from_rgb(18, 92, 64));
    painter.rect_stroke(
        table_rect,
        40.0,
        egui::Stroke::new(4.0, egui::Color32::from_rgb(132, 85, 50)),
        egui::StrokeKind::Outside,
    );

    let c = table_rect.center();

    let board_origin = egui::pos2(c.x - 165.0, c.y - 40.0);
    for i in 0..5 {
        let x = board_origin.x + i as f32 * 68.0;
        let card_rect = egui::Rect::from_min_size(egui::pos2(x, board_origin.y), egui::vec2(58.0, 82.0));
        if let Some(card) = table.cartes_communes.get(i) {
            dessiner_carte(&painter, card_rect, &card.to_string(), true);
        } else {
            dessiner_carte(&painter, card_rect, "", false);
        }
    }

    let pot_rect = egui::Rect::from_center_size(egui::pos2(c.x, c.y + 60.0), egui::vec2(210.0, 48.0));
    painter.rect_filled(pot_rect, 10.0, egui::Color32::from_rgb(11, 41, 30));
    painter.text(
        pot_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("POT  {}", table.pot),
        egui::FontId::proportional(22.0),
        egui::Color32::from_rgb(238, 220, 151),
    );

    let hero_cards_y = table_rect.bottom() - 110.0;
    for i in 0..2 {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 72.0 + i as f32 * 80.0, hero_cards_y),
            egui::vec2(64.0, 92.0),
        );
        if let Some(card) = table.mes_cartes.get(i) {
            dessiner_carte(&painter, card_rect, &card.to_string(), true);
        } else {
            dessiner_carte(&painter, card_rect, "", false);
        }
    }
}

fn dessiner_carte(painter: &egui::Painter, rect: egui::Rect, texte: &str, face_up: bool) {
    if face_up {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(249, 249, 245));
        painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(74, 74, 80)), egui::StrokeKind::Outside);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            texte,
            egui::FontId::proportional(26.0),
            egui::Color32::BLACK,
        );
    } else {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(24, 47, 93));
        painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(112, 148, 220)), egui::StrokeKind::Outside);
    }
}