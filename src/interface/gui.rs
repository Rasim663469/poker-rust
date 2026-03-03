use eframe::egui;
use crate::carte::Carte;
use crate::communication::{MessageServeur, ActionJoueur};

// Écrans de navigation 

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ecran {
    // Menu d'accueil général (liste des jeux disponibles)
    MenuPrincipal,
    // Lobby spécifique au Poker : choisir créer ou rejoindre
    PokerLobby,
    // Écran de configuration avant de créer une partie (nombre joueurs, jetons)
    PokerCreer,
    // Jeu en cours
    Poker,
}

// État réseau partagé 

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

// Application principale 

pub struct CasinoApp {
    ecran: Ecran,
    pub rx_reseau: Option<std::sync::mpsc::Receiver<MessageServeur>>,
    pub tx_reseau: Option<std::sync::mpsc::Sender<ActionJoueur>>,
    pub table: TableReseau,
    // Paramètres de configuration de partie
    pub nb_joueurs: u32,
    pub jetons_depart: u32,
}

impl Default for CasinoApp {
    fn default() -> Self {
        Self {
            ecran: Ecran::MenuPrincipal,
            rx_reseau: None,
            tx_reseau: None,
            table: TableReseau::default(),
            nb_joueurs: 4,
            jetons_depart: 1000,
        }
    }
}

// Boucle principale egui 

impl eframe::App for CasinoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Réception des messages réseau
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
                        self.ecran = Ecran::Poker;
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
                        self.ecran = Ecran::MenuPrincipal;
                    }
                }
            }
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("casino_header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🎰  Casino Rust");
                ui.separator();
                ui.label(self.label_ecran());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.ecran != Ecran::MenuPrincipal {
                        if ui.button("⬅  Menu principal").clicked() {
                            self.ecran = Ecran::MenuPrincipal;
                        }
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.ecran {
                Ecran::MenuPrincipal => ui_menu_principal(ui, &mut self.ecran),
                Ecran::PokerLobby   => ui_poker_lobby(ui, &mut self.ecran),
                Ecran::PokerCreer   => self.ui_poker_creer(ui),
                Ecran::Poker        => self.ui_poker(ui),
            }
        });
    }
}

// Helpers navigation 

impl CasinoApp {
    fn label_ecran(&self) -> &'static str {
        match self.ecran {
            Ecran::MenuPrincipal => "Menu principal",
            Ecran::PokerLobby   => "Poker - Lobby",
            Ecran::PokerCreer   => "Poker - Créer une partie",
            Ecran::Poker        => "Poker - En jeu",
        }
    }

    fn envoyer_action(&mut self, action: ActionJoueur) {
        self.table.a_mon_tour = false;
        if let Some(tx) = &self.tx_reseau {
            let _ = tx.send(action);
        }
    }
}

// Menu principal 
//  Ajouter de nouveaux jeux ici 

fn ui_menu_principal(ui: &mut egui::Ui, ecran: &mut Ecran) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.heading("Jeux disponibles");
        ui.add_space(20.0);
        if ui.button("Poker").clicked() {
            *ecran = Ecran::PokerLobby;
        }
    });
}

// Lobby Poker (créer ou rejoindre) 

fn ui_poker_lobby(ui: &mut egui::Ui, ecran: &mut Ecran) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.heading("Poker");
        ui.add_space(20.0);
        if ui.button("Creer une partie").clicked() {
            *ecran = Ecran::PokerCreer;
        }
        ui.add_space(8.0);
        if ui.button("Rejoindre une partie").clicked() {
            *ecran = Ecran::Poker;
        }
    });
}

// Configuration d'une nouvelle partie poker 

impl CasinoApp {
    fn ui_poker_creer(&mut self, ui: &mut egui::Ui) {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Configuration de la partie").size(24.0).strong());
            ui.add_space(32.0);

            egui::Grid::new("poker_config_grid")
                .num_columns(2)
                .spacing([24.0, 18.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Nombre de joueurs (2-10) :").size(16.0));
                    ui.add(egui::Slider::new(&mut self.nb_joueurs, 2..=10).min_decimals(0));
                    ui.end_row();

                    ui.label(egui::RichText::new("Jetons au départ (100-10 000) :").size(16.0));
                    ui.add(egui::Slider::new(&mut self.jetons_depart, 100..=10_000).min_decimals(0));
                    ui.end_row();
                });

            ui.add_space(40.0);

            if ui.add_sized([220.0, 48.0], egui::Button::new(egui::RichText::new("✅  Lancer la partie").size(18.0))).clicked() {
                self.envoyer_action(ActionJoueur::ConfigurerPartie {
                    nb_joueurs: self.nb_joueurs,
                    jetons: self.jetons_depart,
                });
                self.table.historique.push("Configuration envoyée ! En attente des joueurs...".to_string());
                self.ecran = Ecran::Poker;
            }
        });
    }

// Jeu Poker 

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
}

// Rendu graphique de la table 

fn dessiner_table_reseau(ui: &mut egui::Ui, rect: egui::Rect, table: &TableReseau) {
    let painter = ui.painter_at(rect);
    let bg = egui::Color32::from_rgb(13, 30, 24);
    painter.rect_filled(rect, 18.0, bg);

    let table_rect = rect.shrink2(egui::vec2(18.0, 12.0));
    painter.rect_filled(table_rect, 120.0, egui::Color32::from_rgb(18, 92, 64));
    painter.rect_stroke(
        table_rect,
        120.0,
        egui::Stroke::new(4.0, egui::Color32::from_rgb(132, 85, 50)),
        egui::StrokeKind::Outside,
    );

    let c = table_rect.center();

    // Cartes communes (board)
    let board_origin = egui::pos2(c.x - 165.0, c.y - 20.0);
    for i in 0..5 {
        let x = board_origin.x + i as f32 * 68.0;
        let card_rect = egui::Rect::from_min_size(egui::pos2(x, board_origin.y), egui::vec2(58.0, 82.0));
        if let Some(card) = table.cartes_communes.get(i) {
            dessiner_carte(&painter, card_rect, true);
            poser_image_carte(ui, card_rect.shrink(1.0), &card.image_url_api());
        } else {
            dessiner_carte(&painter, card_rect, false);
        }
    }

    // Pot
    let pot_rect = egui::Rect::from_center_size(egui::pos2(c.x, c.y + 72.0), egui::vec2(210.0, 48.0));
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
        format!("POT  {}", table.pot),
        egui::FontId::proportional(22.0),
        egui::Color32::from_rgb(238, 220, 151),
    );
    dessiner_jetons(&painter, egui::pos2(pot_rect.left() - 34.0, pot_rect.center().y), 3);
    dessiner_jetons(&painter, egui::pos2(pot_rect.right() + 24.0, pot_rect.center().y), 4);

    // Zones joueurs
    dessiner_joueur_zone(
        &painter,
        egui::Rect::from_center_size(egui::pos2(c.x, table_rect.top() + 32.0), egui::vec2(360.0, 54.0)),
        "Adversaire",
    );
    dessiner_joueur_zone(
        &painter,
        egui::Rect::from_center_size(egui::pos2(c.x, table_rect.bottom() - 32.0), egui::vec2(420.0, 54.0)),
        &format!("Toi   |   Stack: {}   |   A payer: {}", table.mes_jetons, table.to_call),
    );

    // Cartes adversaire (dos)
    let bot_cards_y = table_rect.top() + 80.0;
    for i in 0..2 {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 72.0 + i as f32 * 80.0, bot_cards_y),
            egui::vec2(64.0, 92.0),
        );
        dessiner_carte(&painter, card_rect, false);
    }

    // Cartes du joueur local
    let hero_cards_y = table_rect.bottom() - 128.0;
    for i in 0..2 {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 72.0 + i as f32 * 80.0, hero_cards_y),
            egui::vec2(64.0, 92.0),
        );
        if let Some(card) = table.mes_cartes.get(i) {
            dessiner_carte(&painter, card_rect, true);
            poser_image_carte(ui, card_rect.shrink(1.0), &card.image_url_api());
        } else {
            dessiner_carte(&painter, card_rect, false);
        }
    }
}

fn dessiner_joueur_zone(painter: &egui::Painter, rect: egui::Rect, texte: &str) {
    painter.rect_filled(rect, 12.0, egui::Color32::from_rgba_premultiplied(5, 20, 16, 170));
    painter.rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(79, 124, 101)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        texte,
        egui::FontId::proportional(18.0),
        egui::Color32::from_rgb(220, 232, 227),
    );
}

fn dessiner_carte(painter: &egui::Painter, rect: egui::Rect, face_up: bool) {
    if face_up {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(249, 249, 245));
        painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(74, 74, 80)), egui::StrokeKind::Outside);
    } else {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(24, 47, 93));
        painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(112, 148, 220)), egui::StrokeKind::Outside);
        painter.rect_filled(rect.shrink(8.0), 6.0, egui::Color32::from_rgb(38, 62, 111));
    }
}

fn poser_image_carte(ui: &mut egui::Ui, rect: egui::Rect, url: &str) {
    let img = egui::Image::from_uri(url).fit_to_exact_size(rect.size());
    ui.put(rect, img);
}

fn dessiner_jetons(painter: &egui::Painter, center: egui::Pos2, n: usize) {
    for i in 0..n {
        let y = center.y - i as f32 * 6.0;
        let c = egui::pos2(center.x, y);
        painter.circle_filled(c, 12.0, egui::Color32::from_rgb(215, 56, 63));
        painter.circle_stroke(c, 12.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
        painter.circle_stroke(c, 7.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
    }
}