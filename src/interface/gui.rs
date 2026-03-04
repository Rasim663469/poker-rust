use crate::core::cards::{Carte, Paquet};
use crate::core::player::Joueur;
use crate::games::blackjack::engine::{EtatBlackjack, JeuBlackjack};
use crate::games::poker::engine::evaluer_holdem_pour_gui;
use eframe::egui;
use rand::Rng;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EcranCasino {
    Menu,
    Poker,
    Blackjack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TourJoueur {
    Hero,
    Bot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    Terminee,
}

impl Street {
    fn nom(self) -> &'static str {
        match self {
            Street::Preflop => "Preflop",
            Street::Flop => "Flop",
            Street::Turn => "Turn",
            Street::River => "River",
            Street::Showdown => "Showdown",
            Street::Terminee => "Main terminee",
        }
    }
}

struct PokerGuiGame {
    hero: Joueur,
    bot: Joueur,
    paquet: Paquet,
    board: Vec<Carte>,
    pot: u32,
    small_blind: u32,
    big_blind: u32,
    dealer_hero: bool,
    street: Street,
    tour: TourJoueur,
    mise_actuelle: u32,
    besoins_action_hero: bool,
    besoins_action_bot: bool,
    message: String,
    raise_total_input: u32,
}

impl PokerGuiGame {
    fn new(hero_jetons: u32, small_blind: u32, big_blind: u32) -> Self {
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

    fn nouvelle_main(&mut self) {
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

    fn bot_jouer_si_tour(&mut self) {
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

pub struct CasinoApp {
    ecran: EcranCasino,
    jetons_depart: u32,
    small_blind: u32,
    big_blind: u32,
    poker: Option<PokerGuiGame>,
    blackjack: Option<JeuBlackjack>,
    bj_nb_joueurs: u8,
    bj_jetons_depart: u32,
    bj_mise_input: u32,
}

impl Default for CasinoApp {
    fn default() -> Self {
        Self {
            ecran: EcranCasino::Menu,
            jetons_depart: 200,
            small_blind: 10,
            big_blind: 20,
            poker: None,
            blackjack: None,
            bj_nb_joueurs: 3,
            bj_jetons_depart: 500,
            bj_mise_input: 20,
        }
    }
}

impl eframe::App for CasinoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(p) = &mut self.poker {
            p.bot_jouer_si_tour();
        }
        if let Some(bj) = &mut self.blackjack {
            bj.avancer_automatique();
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Casino Rust");
                ui.separator();
                ui.label(match self.ecran {
                    EcranCasino::Menu => "Menu",
                    EcranCasino::Poker => "Poker jouable en GUI",
                    EcranCasino::Blackjack => "Blackjack jouable en GUI",
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.ecran {
            EcranCasino::Menu => self.ui_menu(ui),
            EcranCasino::Poker => self.ui_poker(ui),
            EcranCasino::Blackjack => self.ui_blackjack(ui),
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(80));
    }
}

impl CasinoApp {
    fn ui_menu(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.heading("Menu des jeux");
        ui.label("Jeux disponibles:");
        ui.add_space(10.0);

        if ui.button("Poker Texas Hold'em").clicked() {
            self.ecran = EcranCasino::Poker;
        }
        if ui.button("Blackjack").clicked() {
            self.ecran = EcranCasino::Blackjack;
        }
    }

    fn ui_poker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("<- Retour menu").clicked() {
                self.ecran = EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Poker Heads-up (Toi vs Bot)");
        });

        if self.poker.is_none() {
            ui.add_space(12.0);
            ui.label("Parametres de la table:");
            ui.add(
                egui::DragValue::new(&mut self.jetons_depart)
                    .range(50..=10_000)
                    .prefix("Jetons: "),
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

            if ui.button("Lancer une partie").clicked() {
                self.poker = Some(PokerGuiGame::new(
                    self.jetons_depart,
                    self.small_blind,
                    self.big_blind,
                ));
            }
            return;
        }

        let game = self.poker.as_mut().expect("poker must be Some here");

        ui.separator();
        ui.label(format!("Street: {}", game.street.nom()));
        let board = game
            .board
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        ui.label(format!("Board: {}", board));
        ui.label(format!("Pot: {}", game.pot));
        ui.label(format!("Toi: {} jetons | Bot: {} jetons", game.hero.jetons, game.bot.jetons));
        ui.label(game.message.clone());
        ui.add_space(6.0);

        ui.label(format!(
            "Ta main: {}",
            game.hero
                .main
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ));

        if game.street == Street::Terminee {
            ui.add_space(6.0);
            if ui.button("Nouvelle main").clicked() {
                game.nouvelle_main();
            }
            if ui.button("Fermer la table").clicked() {
                self.poker = None;
            }
            return;
        }

        if game.tour == TourJoueur::Hero && game.besoins_action_hero {
            let a_payer = game.to_call(TourJoueur::Hero);
            ui.label(format!("A payer: {}", a_payer));
            ui.horizontal(|ui| {
                if ui.button(if a_payer == 0 { "Check" } else { "Call" }).clicked() {
                    game.action_check_ou_call(TourJoueur::Hero);
                }
                if ui.button("Fold").clicked() {
                    game.action_fold(TourJoueur::Hero);
                }
            });

            if game.peut_relancer(TourJoueur::Hero) {
                let min = game.total_min_raise();
                let max = game.total_max(TourJoueur::Hero);
                if game.raise_total_input < min {
                    game.raise_total_input = min;
                }
                if game.raise_total_input > max {
                    game.raise_total_input = max;
                }
                ui.add(
                    egui::Slider::new(&mut game.raise_total_input, min..=max)
                        .text("Relance totale"),
                );
                if ui.button("Raise").clicked() {
                    game.action_raise(TourJoueur::Hero, game.raise_total_input);
                }
            }
        } else {
            ui.label("Tour du bot...");
        }
    }

    fn ui_blackjack(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("<- Retour menu").clicked() {
                self.ecran = EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Blackjack");
        });

        if self.blackjack.is_none() {
            ui.add_space(12.0);
            ui.add(
                egui::DragValue::new(&mut self.bj_nb_joueurs)
                    .range(2..=6)
                    .prefix("Joueurs: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.bj_jetons_depart)
                    .range(50..=10_000)
                    .prefix("Jetons: "),
            );
            if ui.button("Creer une table").clicked() {
                self.blackjack = Some(JeuBlackjack::nouveau(
                    self.bj_nb_joueurs as usize,
                    self.bj_jetons_depart,
                ));
            }
            return;
        }

        let mut fermer = false;
        let mut rejouer = false;

        let jeu = self.blackjack.as_mut().expect("blackjack must be Some here");
        ui.separator();
        ui.label(format!("Jetons (toi): {}", jeu.jetons_humain()));
        ui.label(jeu.message.clone());

        let croupier = if jeu.croupier_cachee() {
            if jeu.main_croupier.len() >= 2 {
                format!("?? {}", jeu.main_croupier[1])
            } else {
                "??".to_string()
            }
        } else {
            jeu.main_croupier
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };
        ui.label(format!("Croupier: {} (score {})", croupier, jeu.score_croupier_visible()));

        for (idx, j) in jeu.joueurs.iter().enumerate() {
            let main = if j.main.is_empty() {
                "-".to_string()
            } else {
                j.main
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            ui.label(format!(
                "{}: {} | score {} | jetons {} | mise {}",
                j.nom,
                main,
                jeu.score_joueur(idx),
                j.jetons,
                j.mise
            ));
        }

        match jeu.etat {
            EtatBlackjack::EnAttenteMise | EtatBlackjack::Termine => {
                let max = jeu.jetons_humain().max(1);
                if self.bj_mise_input > max {
                    self.bj_mise_input = max;
                }
                ui.add(egui::Slider::new(&mut self.bj_mise_input, 1..=max).text("Mise"));
                if ui.button("Distribuer").clicked() {
                    let _ = jeu.commencer_manche(self.bj_mise_input);
                }
            }
            EtatBlackjack::TourJoueur => {
                if jeu.est_tour_humain() {
                    ui.horizontal(|ui| {
                        if ui.button("Hit").clicked() {
                            jeu.joueur_hit();
                        }
                        if ui.button("Stand").clicked() {
                            jeu.joueur_stand();
                        }
                    });
                } else {
                    ui.label("Tour des bots...");
                }
            }
            EtatBlackjack::TourCroupier => {
                ui.label("Tour du croupier...");
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Nouvelle table").clicked() {
                fermer = true;
            }
            if ui.button("Rejouer meme table").clicked() {
                rejouer = true;
            }
        });

        if fermer {
            self.blackjack = None;
        } else if rejouer {
            let nb = jeu.joueurs.len();
            let jetons = self.bj_jetons_depart;
            self.blackjack = Some(JeuBlackjack::nouveau(nb, jetons));
        }
    }
}
