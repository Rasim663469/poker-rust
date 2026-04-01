use super::{PokerVue, Street, TourJoueur};
use super::draw::{dessiner_carte, dessiner_joueur_zone, dessiner_jetons};
use super::theme::{back_button, panel_frame, premium_button, section_title, status_panel, subpanel_frame, GOLD_SOFT, TABLE_GREEN, TEXT_DIM};
use crate::core::cards::{Carte, Paquet};
use crate::core::player::Joueur;
use crate::games::poker::engine::evaluer_holdem_pour_gui;
use eframe::egui;
use rand::Rng;
use std::cmp::Ordering;

pub(super) struct PokerGuiGame {
    pub(super) hero: Joueur,
    pub(super) bot: Joueur,
    pub(super) paquet: Paquet,
    pub(super) board: Vec<Carte>,
    pub(super) pot: u32,
    pub(super) small_blind: u32,
    pub(super) big_blind: u32,
    pub(super) dealer_hero: bool,
    pub(super) street: Street,
    pub(super) tour: TourJoueur,
    pub(super) mise_actuelle: u32,
    pub(super) besoins_action_hero: bool,
    pub(super) besoins_action_bot: bool,
    pub(super) message: String,
    pub(super) raise_total_input: u32,
}

impl PokerGuiGame {
    pub(super) fn new(hero_jetons: u32, small_blind: u32, big_blind: u32) -> Self {
        let mut game = Self {
            hero: Joueur::nouveau("Toi".to_string(), hero_jetons),
            bot: Joueur::nouveau("Bot".to_string(), hero_jetons),
            paquet: Paquet::nouveau(),
            board: Vec::new(),
            pot: 0,
            small_blind,
            big_blind,
            dealer_hero: true,
            street: Street::Terminee,
            tour: TourJoueur::Hero,
            mise_actuelle: 0,
            besoins_action_hero: false,
            besoins_action_bot: false,
            message: String::new(),
            raise_total_input: 0,
        };
        game.nouvelle_main();
        game
    }

    pub(super) fn nouvelle_main(&mut self) {
        if self.hero.jetons == 0 || self.bot.jetons == 0 {
            self.street = Street::Terminee;
            self.message = "Partie terminee: un joueur n'a plus de jetons.".to_string();
            return;
        }

        self.paquet = Paquet::nouveau();
        self.paquet.melanger();
        self.board.clear();
        self.pot = 0;
        self.street = Street::Preflop;

        self.hero.main.clear();
        self.bot.main.clear();
        self.hero.couche = false;
        self.bot.couche = false;
        self.hero.mise_tour = 0;
        self.bot.mise_tour = 0;

        self.distribuer_pocket();
        self.poster_blinds();

        self.besoins_action_hero = true;
        self.besoins_action_bot = true;
        self.tour = if self.dealer_hero {
            TourJoueur::Hero
        } else {
            TourJoueur::Bot
        };
        self.raise_total_input = self.mise_actuelle + self.big_blind;
        self.message = format!("Nouvelle main. {}", self.street.nom());
    }

    fn distribuer_pocket(&mut self) {
        for _ in 0..2 {
            if let Some(c) = self.paquet.tirer_carte() {
                self.hero.main.push(c);
            }
            if let Some(c) = self.paquet.tirer_carte() {
                self.bot.main.push(c);
            }
        }
    }

    fn poster_blinds(&mut self) {
        if self.dealer_hero {
            self.prelever_jetons(TourJoueur::Hero, self.small_blind);
            self.prelever_jetons(TourJoueur::Bot, self.big_blind);
            self.mise_actuelle = self.bot.mise_tour;
        } else {
            self.prelever_jetons(TourJoueur::Bot, self.small_blind);
            self.prelever_jetons(TourJoueur::Hero, self.big_blind);
            self.mise_actuelle = self.hero.mise_tour;
        }
        self.message = format!("Blinds postees. Pot: {}", self.pot);
    }

    fn prelever_jetons(&mut self, joueur: TourJoueur, montant: u32) {
        let j = self.joueur_mut(joueur);
        let paye = j.jetons.min(montant);
        j.jetons -= paye;
        j.mise_tour += paye;
        self.pot += paye;
    }

    fn joueur(&self, joueur: TourJoueur) -> &Joueur {
        match joueur {
            TourJoueur::Hero => &self.hero,
            TourJoueur::Bot => &self.bot,
        }
    }

    fn joueur_mut(&mut self, joueur: TourJoueur) -> &mut Joueur {
        match joueur {
            TourJoueur::Hero => &mut self.hero,
            TourJoueur::Bot => &mut self.bot,
        }
    }

    fn autre(j: TourJoueur) -> TourJoueur {
        match j {
            TourJoueur::Hero => TourJoueur::Bot,
            TourJoueur::Bot => TourJoueur::Hero,
        }
    }

    fn to_call(&self, joueur: TourJoueur) -> u32 {
        self.mise_actuelle.saturating_sub(self.joueur(joueur).mise_tour)
    }

    fn total_min_raise(&self) -> u32 {
        if self.mise_actuelle == 0 {
            self.big_blind
        } else {
            self.mise_actuelle + self.big_blind
        }
    }

    fn total_max(&self, joueur: TourJoueur) -> u32 {
        self.joueur(joueur).mise_tour + self.joueur(joueur).jetons
    }

    fn peut_relancer(&self, joueur: TourJoueur) -> bool {
        let to_call = self.to_call(joueur);
        self.joueur(joueur).jetons > to_call && self.total_max(joueur) >= self.total_min_raise()
    }

    fn action_fold(&mut self, joueur: TourJoueur) {
        self.joueur_mut(joueur).couche = true;
        self.set_besoin_action(joueur, false);
        self.message = format!("{} se couche.", self.joueur(joueur).nom);
        self.verifier_fin_main_ou_tour();
    }

    fn action_check_ou_call(&mut self, joueur: TourJoueur) {
        let a_payer = self.to_call(joueur);
        self.prelever_jetons(joueur, a_payer);
        self.set_besoin_action(joueur, false);
        if a_payer == 0 {
            self.message = format!("{} check.", self.joueur(joueur).nom);
        } else {
            self.message = format!("{} suit {}.", self.joueur(joueur).nom, a_payer);
        }
        self.verifier_fin_main_ou_tour();
    }

    fn action_raise(&mut self, joueur: TourJoueur, total: u32) {
        let total_min = self.total_min_raise();
        let total_max = self.total_max(joueur);
        if total < total_min || total > total_max {
            self.message = format!(
                "Relance invalide: attendu entre {} et {}",
                total_min, total_max
            );
            return;
        }
        let delta = total.saturating_sub(self.joueur(joueur).mise_tour);
        self.prelever_jetons(joueur, delta);
        self.joueur_mut(joueur).mise_tour = total;
        self.mise_actuelle = total;

        self.set_besoin_action(joueur, false);
        let autre = Self::autre(joueur);
        if !self.joueur(autre).couche && self.joueur(autre).jetons > 0 {
            self.set_besoin_action(autre, true);
        }
        self.message = format!("{} relance a {}.", self.joueur(joueur).nom, total);
        self.avancer_tour();
    }

    fn set_besoin_action(&mut self, joueur: TourJoueur, valeur: bool) {
        match joueur {
            TourJoueur::Hero => self.besoins_action_hero = valeur,
            TourJoueur::Bot => self.besoins_action_bot = valeur,
        }
    }

    fn besoin_action(&self, joueur: TourJoueur) -> bool {
        match joueur {
            TourJoueur::Hero => self.besoins_action_hero,
            TourJoueur::Bot => self.besoins_action_bot,
        }
    }

    fn actifs_non_couches(&self) -> u8 {
        let mut c = 0;
        if !self.hero.couche {
            c += 1;
        }
        if !self.bot.couche {
            c += 1;
        }
        c
    }

    fn verifier_fin_main_ou_tour(&mut self) {
        if self.actifs_non_couches() == 1 {
            let gagnant = if !self.hero.couche {
                TourJoueur::Hero
            } else {
                TourJoueur::Bot
            };
            self.joueur_mut(gagnant).jetons += self.pot;
            self.message = format!(
                "{} gagne le pot de {} (abandon).",
                self.joueur(gagnant).nom,
                self.pot
            );
            self.pot = 0;
            self.street = Street::Terminee;
            self.dealer_hero = !self.dealer_hero;
            return;
        }

        if !self.besoins_action_hero && !self.besoins_action_bot {
            self.avancer_street();
            return;
        }
        self.avancer_tour();
    }

    fn avancer_tour(&mut self) {
        let autre = Self::autre(self.tour);
        if self.besoin_action(autre) && !self.joueur(autre).couche && self.joueur(autre).jetons > 0 {
            self.tour = autre;
            return;
        }
        if self.besoin_action(self.tour)
            && !self.joueur(self.tour).couche
            && self.joueur(self.tour).jetons > 0
        {
            return;
        }
        self.verifier_fin_main_ou_tour();
    }

    fn avancer_street(&mut self) {
        self.hero.mise_tour = 0;
        self.bot.mise_tour = 0;
        self.mise_actuelle = 0;
        self.raise_total_input = self.big_blind;

        self.besoins_action_hero = !self.hero.couche && self.hero.jetons > 0;
        self.besoins_action_bot = !self.bot.couche && self.bot.jetons > 0;

        self.street = match self.street {
            Street::Preflop => {
                self.bruler();
                self.tirer_board(3);
                Street::Flop
            }
            Street::Flop => {
                self.bruler();
                self.tirer_board(1);
                Street::Turn
            }
            Street::Turn => {
                self.bruler();
                self.tirer_board(1);
                Street::River
            }
            Street::River => Street::Showdown,
            Street::Showdown | Street::Terminee => self.street,
        };

        if self.street == Street::Showdown {
            self.executer_showdown();
            self.dealer_hero = !self.dealer_hero;
            return;
        }

        self.tour = if self.dealer_hero {
            TourJoueur::Bot
        } else {
            TourJoueur::Hero
        };
        self.message = format!("{}.", self.street.nom());
        if !self.besoin_action(self.tour) {
            self.tour = Self::autre(self.tour);
        }
    }

    fn bruler(&mut self) {
        let _ = self.paquet.tirer_carte();
    }

    fn tirer_board(&mut self, n: usize) {
        for _ in 0..n {
            if let Some(c) = self.paquet.tirer_carte() {
                self.board.push(c);
            }
        }
    }

    fn executer_showdown(&mut self) {
        let mut hero_cartes = self.hero.main.clone();
        hero_cartes.extend(self.board.iter().copied());
        let mut bot_cartes = self.bot.main.clone();
        bot_cartes.extend(self.board.iter().copied());

        let hero_eval = evaluer_holdem_pour_gui(&hero_cartes);
        let bot_eval = evaluer_holdem_pour_gui(&bot_cartes);

        let cmp = comparer_eval((&hero_eval.0, &hero_eval.1), (&bot_eval.0, &bot_eval.1));
        match cmp {
            Ordering::Greater => {
                self.hero.jetons += self.pot;
                self.message = format!("Showdown: toi gagnes avec {}.", hero_eval.2);
            }
            Ordering::Less => {
                self.bot.jetons += self.pot;
                self.message = format!("Showdown: bot gagne avec {}.", bot_eval.2);
            }
            Ordering::Equal => {
                let half = self.pot / 2;
                let rest = self.pot % 2;
                self.hero.jetons += half + rest;
                self.bot.jetons += half;
                self.message = format!("Showdown: egalite ({}).", hero_eval.2);
            }
        }
        self.pot = 0;
        self.street = Street::Terminee;
    }

    pub(super) fn bot_jouer_si_tour(&mut self) {
        if self.street == Street::Terminee
            || self.street == Street::Showdown
            || self.tour != TourJoueur::Bot
        {
            return;
        }
        if !self.besoins_action_bot || self.bot.couche || self.bot.jetons == 0 {
            return;
        }

        let to_call = self.to_call(TourJoueur::Bot);
        let mut rng = rand::thread_rng();

        if to_call == 0 {
            if self.peut_relancer(TourJoueur::Bot) && rng.gen_bool(0.20) {
                let min_raise = self.total_min_raise();
                let max_raise = self.total_max(TourJoueur::Bot);
                let target = (min_raise + self.big_blind).min(max_raise);
                self.action_raise(TourJoueur::Bot, target);
            } else {
                self.action_check_ou_call(TourJoueur::Bot);
            }
            return;
        }

        let stack = self.bot.jetons;
        let ratio = to_call as f32 / stack.max(1) as f32;
        if ratio > 0.45 && rng.gen_bool(0.65) {
            self.action_fold(TourJoueur::Bot);
            return;
        }

        if self.peut_relancer(TourJoueur::Bot) && ratio < 0.20 && rng.gen_bool(0.15) {
            let min_raise = self.total_min_raise();
            let max_raise = self.total_max(TourJoueur::Bot);
            let target = (min_raise + self.big_blind).min(max_raise);
            self.action_raise(TourJoueur::Bot, target);
            return;
        }

        self.action_check_ou_call(TourJoueur::Bot);
    }
}

fn comparer_eval(a: (&u8, &[u8]), b: (&u8, &[u8])) -> Ordering {
    a.0.cmp(b.0).then_with(|| a.1.cmp(b.1))
}

impl super::CasinoApp {
    pub(super) fn ui_poker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if back_button(ui, "<- Retour menu").clicked() {
                self.ecran = super::EcranCasino::Menu;
                self.poker_vue = PokerVue::Choix;
            }
            ui.separator();
            ui.heading("Poker Texas Hold'em");
        });
        ui.separator();
        match self.poker_vue {
            PokerVue::Choix => self.ui_poker_choix(ui),
            PokerVue::Solo => self.ui_poker_solo(ui),
            PokerVue::Online => self.ui_poker_online(ui),
        }
    }

    pub(super) fn ui_poker_choix(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            section_title(ui, "Choisis un mode", "Texas Hold'em local ou multijoueur.");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("Mode Solo (vs Bots)").min_size(egui::vec2(240.0, 50.0)))
                    .clicked()
                {
                    self.poker_vue = PokerVue::Solo;
                }
                if ui
                    .add(
                        egui::Button::new("Mode Online (multijoueur)")
                            .min_size(egui::vec2(260.0, 50.0)),
                    )
                    .clicked()
                {
                    self.poker_vue = PokerVue::Online;
                }
            });
        });
    }

    pub(super) fn ui_poker_solo(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if back_button(ui, "< Retour choix mode").clicked() {
                self.poker_vue = PokerVue::Choix;
            }
            ui.separator();
            ui.label("Heads-up (Toi vs Bot)");
        });

        if self.poker.is_none() {
            panel_frame().show(ui, |ui| {
                section_title(ui, "Table Poker Solo", "Prépare une table heads-up contre le bot.");
                ui.add_space(10.0);
                subpanel_frame().show(ui, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.jetons_depart)
                            .range(50..=10_000)
                            .prefix("Jetons depart: ")
                            .speed(10.0),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.small_blind)
                            .range(1..=1_000)
                            .prefix("SB: "),
                    );
                    if self.big_blind <= self.small_blind {
                        self.big_blind = self.small_blind + 1;
                    }
                    ui.add(
                        egui::DragValue::new(&mut self.big_blind)
                            .range((self.small_blind + 1)..=5_000)
                            .prefix("BB: "),
                    );
                });

                ui.add_space(10.0);
                if premium_button(ui, "Lancer une partie").clicked()
                {
                    self.poker = Some(PokerGuiGame::new(
                        self.jetons_depart,
                        self.small_blind,
                        self.big_blind,
                    ));
                }
            });
            return;
        }

        let game = self.poker.as_mut().expect("poker must be Some here");

        let mut fermer_table = false;
        ui.separator();
        ui.label(
            egui::RichText::new(format!("Street: {}", game.street.nom()))
                .size(20.0)
                .strong()
                .color(GOLD_SOFT),
        );

        ui.add_space(8.0);
        let table_height = 430.0;
        let table_width = (ui.available_width() - 12.0).max(760.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(table_width, table_height), egui::Sense::hover());
        dessiner_table_solo(ui, rect, game);

        ui.add_space(10.0);
        status_panel(
            ui,
            format!(
                "Pot: {} | Toi: {} jetons | Bot: {} jetons | Mise actuelle: {}",
                game.pot, game.hero.jetons, game.bot.jetons, game.mise_actuelle
            ),
        );
        ui.add_space(6.0);
        subpanel_frame().show(ui, |ui| {
            ui.label(egui::RichText::new(&game.message).color(GOLD_SOFT));
        });

        ui.add_space(10.0);
        if game.street == Street::Terminee {
            ui.horizontal(|ui| {
                if ui.button("Nouvelle main").clicked() {
                    game.nouvelle_main();
                }
                if ui.button("Retour menu Poker").clicked() {
                    fermer_table = true;
                }
            });
            if fermer_table {
                self.banque_joueur += game.hero.jetons;
                self.poker = None;
            }
            return;
        }

        ui.horizontal(|ui| {
            if ui.button("Quitter la table").clicked() {
                fermer_table = true;
            }
        });

        if fermer_table {
            self.banque_joueur += game.hero.jetons;
            self.poker = None;
            return;
        }

        if game.tour != TourJoueur::Hero || !game.besoins_action_hero || game.hero.couche {
            ui.label(egui::RichText::new("Attends l'action du bot...").color(TEXT_DIM));
            return;
        }

        let to_call = game.to_call(TourJoueur::Hero);
        subpanel_frame().show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Ton tour. Mise tour: {} | A payer: {}",
                    game.hero.mise_tour, to_call
                ))
                .color(GOLD_SOFT),
            );

            ui.horizontal(|ui| {
                if ui.button("Fold").clicked() {
                    game.action_fold(TourJoueur::Hero);
                }

                let lib_call = if to_call == 0 { "Check" } else { "Call" };
                if ui.button(lib_call).clicked() {
                    game.action_check_ou_call(TourJoueur::Hero);
                }
            });

            if game.peut_relancer(TourJoueur::Hero) {
                let min_raise = game.total_min_raise();
                let max_raise = game.total_max(TourJoueur::Hero);
                if game.raise_total_input < min_raise || game.raise_total_input > max_raise {
                    game.raise_total_input = min_raise;
                }

                ui.add_space(6.0);
                ui.label(format!("Relance totale ({}..={}):", min_raise, max_raise));
                ui.add(egui::Slider::new(
                    &mut game.raise_total_input,
                    min_raise..=max_raise,
                ));
                if ui.button("Raise").clicked() {
                    game.action_raise(TourJoueur::Hero, game.raise_total_input);
                }
            }
        });
    }
}

fn dessiner_table_solo(ui: &mut egui::Ui, rect: egui::Rect, game: &PokerGuiGame) {
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
        let card = game.board.get(i);
        dessiner_carte(ui, &painter, card_rect, card, card.is_some());
    }

    let pot_rect = egui::Rect::from_center_size(egui::pos2(c.x, c.y + 62.0), egui::vec2(180.0, 46.0));
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
        format!("POT  {}", game.pot),
        egui::FontId::proportional(22.0),
        egui::Color32::from_rgb(238, 220, 151),
    );
    dessiner_jetons(&painter, egui::pos2(pot_rect.left() - 34.0, pot_rect.center().y), 3);
    dessiner_jetons(&painter, egui::pos2(pot_rect.right() + 24.0, pot_rect.center().y), 4);

    let bot_name = if game.tour == TourJoueur::Bot { "Bot - son tour" } else { "Bot" };
    dessiner_joueur_zone(
        &painter,
        egui::Rect::from_center_size(
            egui::pos2(c.x, table_rect.top() + 40.0),
            egui::vec2(280.0, 56.0),
        ),
        bot_name,
        game.bot.jetons,
        game.bot.mise_tour,
    );
    let hero_name = if game.tour == TourJoueur::Hero { "Toi - ton tour" } else { "Toi" };
    dessiner_joueur_zone(
        &painter,
        egui::Rect::from_center_size(
            egui::pos2(c.x, table_rect.bottom() - 40.0),
            egui::vec2(280.0, 56.0),
        ),
        hero_name,
        game.hero.jetons,
        game.hero.mise_tour,
    );

    let bot_cards_y = table_rect.top() + 78.0;
    for i in 0..2 {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 62.0 + i as f32 * 68.0, bot_cards_y),
            egui::vec2(58.0, 84.0),
        );
        let reveal = game.street == Street::Terminee || game.street == Street::Showdown;
        if reveal {
            let card = game.bot.main.get(i);
            dessiner_carte(ui, &painter, card_rect, card, card.is_some());
        } else {
            dessiner_carte(ui, &painter, card_rect, None, false);
        }
    }

    let hero_cards_y = table_rect.bottom() - 164.0;
    for i in 0..2 {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 62.0 + i as f32 * 68.0, hero_cards_y),
            egui::vec2(58.0, 84.0),
        );
        let card = game.hero.main.get(i);
        dessiner_carte(ui, &painter, card_rect, card, card.is_some());
    }
}
