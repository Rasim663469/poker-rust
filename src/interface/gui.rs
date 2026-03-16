use crate::core::cards::{Carte, Paquet};
use crate::core::player::Joueur;
use crate::games::blackjack::engine::{EtatBlackjack, JeuBlackjack};
use crate::games::crash::engine::{EtatCrash, JeuCrash};
use crate::games::mines::engine::{CaseMine, EtatMines, JeuMines};
use crate::games::poker::engine::evaluer_holdem_pour_gui;
use crate::network::protocol::{ActionJoueur, MessageClient, MessageServeur};
use crate::network::{recv_json, send_json};
use eframe::egui;
use rand::Rng;
use std::cmp::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use tokio::net::TcpStream;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EcranCasino {
    Menu,
    Poker,
    Blackjack,
    Mines,
    Crash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PokerVue {
    Choix,
    Solo,
    Online,
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

struct OnlinePokerState {
    adresse: String,
    pseudo: String,
    est_hote: bool,
    nb_joueurs: u32,
    jetons_depart: u32,
    connecte: bool,
    statut: String,
    logs: Vec<String>,
    main: Vec<Carte>,
    board: Vec<Carte>,
    pot: u32,
    en_attente_action: bool,
    to_call: u32,
    peut_relancer: bool,
    jetons_restants: u32,
    raise_total_input: u32,
}

impl Default for OnlinePokerState {
    fn default() -> Self {
        Self {
            adresse: "127.0.0.1:8080".to_string(),
            pseudo: "Joueur".to_string(),
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

pub struct CasinoApp {
    ecran: EcranCasino,
    poker_vue: PokerVue,
    jetons_depart: u32,
    small_blind: u32,
    big_blind: u32,
    poker: Option<PokerGuiGame>,
    poker_online: OnlinePokerState,
    tx_online: Option<mpsc::Sender<ActionJoueur>>,
    rx_online: Option<mpsc::Receiver<MessageServeur>>,
    blackjack: Option<JeuBlackjack>,
    bj_nb_joueurs: u8,
    bj_jetons_depart: u32,
    bj_mise_input: u32,
    // Mines
    mines: Option<JeuMines>,
    mines_nb_mines: u8,
    mines_mise_input: f64,
    mines_graine_client: String,
    mines_autoplay_count: u8,
    mines_nonce: u64,
    mines_solde: f64,
    mines_ui_erreur: String,
    mines_paiement_applique: bool,
    // Crash
    crash: JeuCrash,
    crash_mise_input: f64,
    crash_auto_cashout: f64,
    crash_auto_actif: bool,
    crash_solde: f64,
    crash_ui_erreur: String,
    crash_last_tick: Instant,
}

impl Default for CasinoApp {
    fn default() -> Self {
        Self {
            ecran: EcranCasino::Menu,
            poker_vue: PokerVue::Choix,
            jetons_depart: 200,
            small_blind: 10,
            big_blind: 20,
            poker: None,
            poker_online: OnlinePokerState::default(),
            tx_online: None,
            rx_online: None,
            blackjack: None,
            bj_nb_joueurs: 3,
            bj_jetons_depart: 500,
            bj_mise_input: 20,
            // Mines
            mines: None,
            mines_nb_mines: 3,
            mines_mise_input: 50.0,
            mines_graine_client: "player123".to_string(),
            mines_autoplay_count: 3,
            mines_nonce: 1,
            mines_solde: 2000.0,
            mines_ui_erreur: String::new(),
            mines_paiement_applique: true,
            // Crash
            crash: JeuCrash::nouveau(),
            crash_mise_input: 50.0,
            crash_auto_cashout: 2.0,
            crash_auto_actif: false,
            crash_solde: 2000.0,
            crash_ui_erreur: String::new(),
            crash_last_tick: Instant::now(),
        }
    }
}

impl eframe::App for CasinoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pomper_messages_online();

        if let Some(p) = &mut self.poker {
            p.bot_jouer_si_tour();
        }
        if let Some(bj) = &mut self.blackjack {
            bj.avancer_automatique();
        }

        let maintenant = Instant::now();
        let delta = (maintenant - self.crash_last_tick).as_secs_f32();
        self.crash_last_tick = maintenant;
        if self.crash.manche_en_cours() {
            self.crash.avancer(delta);
            if self.crash_auto_actif
                && self.crash.est_en_vol()
                && self.crash.multiplicateur_vol() >= self.crash_auto_cashout.max(1.01)
            {
                if let Ok(paiement) = self.crash.encaisser_a(self.crash_auto_cashout.max(1.01)) {
                    self.crash_solde += paiement;
                }
            }
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Casino Rust");
                ui.separator();
                ui.label(match self.ecran {
                    EcranCasino::Menu => "Menu",
                    EcranCasino::Poker => "Poker jouable en GUI",
                    EcranCasino::Blackjack => "Blackjack jouable en GUI",
                    EcranCasino::Mines => "Mines jouable en GUI",
                    EcranCasino::Crash => "Crash jouable en GUI",
                });
                if self.ecran == EcranCasino::Mines {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Balance Mines: {:.2}", self.mines_solde))
                            .strong()
                            .color(egui::Color32::from_rgb(247, 211, 88)),
                    );
                }
                if self.ecran == EcranCasino::Crash {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Balance Crash: {:.2}", self.crash_solde))
                            .strong()
                            .color(egui::Color32::from_rgb(247, 211, 88)),
                    );
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.ecran {
            EcranCasino::Menu => self.ui_menu(ui),
            EcranCasino::Poker => self.ui_poker(ui),
            EcranCasino::Blackjack => self.ui_blackjack(ui),
            EcranCasino::Mines => self.ui_mines(ui),
            EcranCasino::Crash => self.ui_crash(ui),
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(80));
    }
}

impl CasinoApp {
    fn pomper_messages_online(&mut self) {
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
                }
            }
        }
    }

    fn ui_menu(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.heading("Menu des jeux");
        ui.label("Jeux disponibles:");
        ui.add_space(10.0);

        if ui.button("Poker Texas Hold'em").clicked() {
            self.ecran = EcranCasino::Poker;
            self.poker_vue = PokerVue::Choix;
        }
        if ui.button("Blackjack").clicked() {
            self.ecran = EcranCasino::Blackjack;
        }
        if ui.button("Mines").clicked() {
            self.ecran = EcranCasino::Mines;
        }
        if ui.button("Crash").clicked() {
            self.ecran = EcranCasino::Crash;
        }
    }

    fn ui_poker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("<- Retour menu").clicked() {
                self.ecran = EcranCasino::Menu;
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

    fn ui_poker_choix(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.heading("Choisis un mode");
        ui.add_space(6.0);
        if ui.button("Mode Solo (vs Bots)").clicked() {
            self.poker_vue = PokerVue::Solo;
        }
        if ui.button("Mode Online (multijoueur)").clicked() {
            self.poker_vue = PokerVue::Online;
        }
    }

    fn ui_poker_solo(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("< Retour choix mode").clicked() {
                self.poker_vue = PokerVue::Choix;
            }
            ui.separator();
            ui.label("Heads-up (Toi vs Bot)");
        });

        if self.poker.is_none() {
            ui.add_space(14.0);
            ui.heading("Menu Poker Solo");
            ui.label("Parametres de la table:");
            ui.add_space(8.0);

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

            ui.add_space(10.0);
            if ui.button("Lancer une partie").clicked() {
                self.poker = Some(PokerGuiGame::new(
                    self.jetons_depart,
                    self.small_blind,
                    self.big_blind,
                ));
            }
            return;
        }

        let Some(game) = &mut self.poker else {
            return;
        };
        let mut fermer_table = false;
        ui.separator();
        ui.label(format!("Street: {}", game.street.nom()));

        ui.add_space(8.0);
        let table_height = 430.0;
        let table_width = (ui.available_width() - 12.0).max(760.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(table_width, table_height), egui::Sense::hover());
        dessiner_table_solo(ui, rect, game);

        ui.add_space(10.0);
        ui.monospace(format!(
            "Pot: {} | Toi: {} jetons | Bot: {} jetons | Mise actuelle: {}",
            game.pot, game.hero.jetons, game.bot.jetons, game.mise_actuelle
        ));
        ui.monospace(&game.message);

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
            self.poker = None;
            return;
        }

        if game.tour != TourJoueur::Hero || !game.besoins_action_hero || game.hero.couche {
            ui.label("Attends l'action du bot...");
            return;
        }

        let to_call = game.to_call(TourJoueur::Hero);
        ui.label(format!(
            "Ton tour. Mise tour: {} | A payer: {}",
            game.hero.mise_tour, to_call
        ));

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
    }

    fn ui_poker_online(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("< Retour choix mode").clicked() {
                self.poker_vue = PokerVue::Choix;
            }
            ui.separator();
            ui.label("Multijoueur TCP");
        });

        if !self.poker_online.connecte {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Adresse:");
                ui.text_edit_singleline(&mut self.poker_online.adresse);
            });
            ui.horizontal(|ui| {
                ui.label("Pseudo:");
                ui.text_edit_singleline(&mut self.poker_online.pseudo);
            });
            ui.checkbox(&mut self.poker_online.est_hote, "Je suis l'hote");
            if self.poker_online.est_hote {
                ui.add(
                    egui::Slider::new(&mut self.poker_online.nb_joueurs, 2..=6)
                        .text("Nombre de joueurs"),
                );
                ui.add(
                    egui::Slider::new(&mut self.poker_online.jetons_depart, 50..=10_000)
                        .text("Jetons de depart"),
                );
            }

            if ui.button("Se connecter").clicked() {
                self.demarrer_client_online();
            }
            ui.label(format!("Statut: {}", self.poker_online.statut));
            return;
        }

        ui.horizontal(|ui| {
            if ui.button("Se deconnecter").clicked() {
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

        ui.monospace(format!(
            "Pot: {} | Jetons: {}",
            self.poker_online.pot, self.poker_online.jetons_restants
        ));

        if self.poker_online.en_attente_action {
            ui.separator();
            ui.label(format!("Ton tour. A payer: {}", self.poker_online.to_call));
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
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for l in self.poker_online.logs.iter().rev().take(60).rev() {
                    ui.label(l);
                }
            });
    }

    fn demarrer_client_online(&mut self) {
        if self.poker_online.connecte {
            return;
        }

        let adresse = self.poker_online.adresse.clone();
        let pseudo = self.poker_online.pseudo.clone();
        let est_hote = self.poker_online.est_hote;
        let nb_joueurs = self.poker_online.nb_joueurs;
        let jetons_depart = self.poker_online.jetons_depart;

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

                let hello = MessageClient::Connexion { pseudo };
                if send_json(&mut stream, &hello).await.is_err() {
                    let _ = tx_srv_to_ui.send(MessageServeur::Erreur {
                        message: "Echec envoi du message de connexion.".to_string(),
                    });
                    return;
                }

                let _ = tx_srv_to_ui.send(MessageServeur::Bienvenue {
                    message: "Connecte au serveur.".to_string(),
                });

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

    fn arreter_client_online(&mut self) {
        self.tx_online = None;
        self.rx_online = None;
        self.poker_online.connecte = false;
        self.poker_online.en_attente_action = false;
        self.poker_online.statut = "Deconnecte".to_string();
    }

    fn envoyer_action_online(&mut self, action: ActionJoueur) {
        if let Some(tx) = &self.tx_online {
            if tx.send(action).is_ok() {
                self.poker_online.en_attente_action = false;
                self.poker_online.statut = "Action envoyee".to_string();
            } else {
                self.poker_online.statut = "Echec envoi action".to_string();
            }
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
            ui.add_space(10.0);
            ui.heading("Menu Blackjack");
            ui.add(
                egui::Slider::new(&mut self.bj_nb_joueurs, 2..=6)
                    .text("Joueurs totaux (toi inclus)"),
            );
            ui.add(
                egui::DragValue::new(&mut self.bj_jetons_depart)
                    .range(50..=50_000)
                    .prefix("Jetons depart: ")
                    .speed(10.0),
            );
            if ui.button("Creer table Blackjack").clicked() {
                self.blackjack = Some(JeuBlackjack::nouveau(
                    self.bj_nb_joueurs as usize,
                    self.bj_jetons_depart,
                ));
                self.bj_mise_input = 20.min(self.bj_jetons_depart.max(1));
            }
            return;
        }

        let Some(jeu) = &mut self.blackjack else {
            return;
        };
        ui.separator();
        ui.label(format!(
            "Jetons (toi): {} | Mise de reference: {}",
            jeu.jetons_humain(),
            jeu.mise_reference
        ));
        ui.label(&jeu.message);
        ui.add_space(8.0);

        let table_height = 500.0;
        let table_width = (ui.available_width() - 4.0).max(760.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(table_width, table_height), egui::Sense::hover());
        dessiner_table_blackjack(ui, rect, jeu);

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);
        ui.group(|ui| {
            if jeu.etat == EtatBlackjack::EnAttenteMise || jeu.etat == EtatBlackjack::Termine {
                let max_mise = jeu.jetons_humain().max(1);
                if self.bj_mise_input == 0 || self.bj_mise_input > max_mise {
                    self.bj_mise_input = 1.min(max_mise);
                }
                ui.label("Nouvelle manche:");
                ui.add(egui::Slider::new(&mut self.bj_mise_input, 1..=max_mise).text("Mise"));
                if ui.button("Distribuer").clicked() {
                    let _ = jeu.commencer_manche(self.bj_mise_input);
                }
            } else if jeu.etat == EtatBlackjack::TourJoueur && jeu.est_tour_humain() {
                ui.label("Ton tour");
                ui.horizontal(|ui| {
                    if ui.button("Hit").clicked() {
                        jeu.joueur_hit();
                    }
                    if ui.button("Stand").clicked() {
                        jeu.joueur_stand();
                    }
                });
            } else {
                ui.label("Tour des bots / croupier...");
            }
        });
    }

    // ─── Crash ──────────────────────────────────────────────────────────

    fn ui_crash(&mut self, ui: &mut egui::Ui) {
        let anim_t = ui.ctx().input(|i| i.time) as f32;

        ui.horizontal(|ui| {
            let btn_retour = crash_button_style(
                egui::RichText::new("<- Retour menu")
                    .strong()
                    .color(egui::Color32::from_rgb(225, 232, 238)),
                egui::Color32::from_rgb(148, 163, 184),
                egui::vec2(170.0, 36.0),
            );
            if ui.add(btn_retour).clicked() {
                self.ecran = EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Crash");
            ui.separator();
            ui.label("Sors avant l'explosion");
        });

        ui.separator();
        let payout_affiche = match self.crash.etat {
            EtatCrash::Encaisse { paiement } => paiement,
            EtatCrash::Explose { .. } => 0.0,
            EtatCrash::EnVol => self.crash.mise * self.crash.multiplicateur,
            EtatCrash::EnAttente => 0.0,
        };
        let multiplicateur_accent = match self.crash.etat {
            EtatCrash::Explose { .. } => egui::Color32::from_rgb(231, 76, 60),
            EtatCrash::Encaisse { .. } => egui::Color32::from_rgb(46, 204, 113),
            _ => egui::Color32::from_rgb(46, 204, 113),
        };
        let payout_accent = match self.crash.etat {
            EtatCrash::Explose { .. } => egui::Color32::from_rgb(231, 76, 60),
            EtatCrash::Encaisse { .. } => egui::Color32::from_rgb(46, 204, 113),
            _ => egui::Color32::from_rgb(245, 176, 65),
        };

        let controls_fixed_h = 186.0;
        let top_area_h = (ui.available_height() - controls_fixed_h - 8.0).max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), top_area_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let zone_w = ui.available_width();
                let kpi_h = 74.0;
                let mult_h = 58.0;
                let info_h = 56.0;
                let hist_h = 74.0;

                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, kpi_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.columns(4, |cols| {
                            ui_crash_indicateur(
                                &mut cols[0],
                                "BALANCE",
                                format!("{:.2}", self.crash_solde),
                                egui::Color32::from_rgb(247, 211, 88),
                            );
                            ui_crash_indicateur(
                                &mut cols[1],
                                "MISE",
                                format!("{:.2}", self.crash_mise_input),
                                egui::Color32::from_rgb(93, 173, 226),
                            );
                            ui_crash_indicateur(
                                &mut cols[2],
                                "MULTIPLICATEUR",
                                format!("{:.2}x", self.crash.multiplicateur),
                                multiplicateur_accent,
                            );
                            ui_crash_indicateur(
                                &mut cols[3],
                                "PAYOUT",
                                format!("{:.2}", payout_affiche),
                                payout_accent,
                            );
                        });
                    },
                );

                ui.add_space(4.0);
                let pulsation = if self.crash.est_en_vol() {
                    0.35 + 0.65 * (anim_t * 6.0).sin().abs()
                } else {
                    0.0
                };
                let mult_color = match self.crash.etat {
                    EtatCrash::Explose { .. } => egui::Color32::from_rgb(231, 76, 60),
                    EtatCrash::Encaisse { .. } => egui::Color32::from_rgb(46, 204, 113),
                    _ if self.crash.multiplicateur >= 2.0 => egui::Color32::from_rgb(
                        80,
                        (200.0 + 55.0 * pulsation) as u8,
                        (120.0 + 90.0 * pulsation) as u8,
                    ),
                    _ => egui::Color32::from_rgb(
                        (200.0 + 55.0 * pulsation) as u8,
                        (90.0 + 40.0 * pulsation) as u8,
                        90,
                    ),
                };
                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, mult_h),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.2}x", self.crash.multiplicateur))
                                .size(40.0)
                                .strong()
                                .color(mult_color),
                        );
                    },
                );

                ui.add_space(4.0);
                let graph_h = (ui.available_height() - info_h - hist_h - 8.0).max(96.0);
                let graph_w = zone_w.max(500.0);
                let (graph_rect, _) =
                    ui.allocate_exact_size(egui::vec2(graph_w, graph_h), egui::Sense::hover());
                dessiner_graphique_crash(ui, graph_rect, &self.crash);

                ui.add_space(6.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, info_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if !self.crash.message.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.crash.message)
                                    .size(17.0)
                                    .strong()
                                    .color(match self.crash.etat {
                                        EtatCrash::Explose { .. } => {
                                            egui::Color32::from_rgb(231, 76, 60)
                                        }
                                        EtatCrash::Encaisse { .. } => {
                                            egui::Color32::from_rgb(46, 204, 113)
                                        }
                                        _ => egui::Color32::from_rgb(220, 232, 227),
                                    }),
                            );
                        } else {
                            ui.add_space(22.0);
                        }

                        ui.horizontal(|ui| {
                            if let Some(point) = self.crash.point_crash_revele() {
                                ui.monospace(format!("Point d'explosion: {:.2}x", point));
                            }
                            if !self.crash_ui_erreur.is_empty() {
                                ui.colored_label(
                                    egui::Color32::from_rgb(231, 76, 60),
                                    &self.crash_ui_erreur,
                                );
                            }
                        });
                    },
                );

                ui.add_space(2.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, hist_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Historique (ligne)").strong());
                                ui.separator();
                                ui.colored_label(egui::Color32::from_rgb(231, 76, 60), "< 2.00x");
                                ui.separator();
                                ui.colored_label(
                                    egui::Color32::from_rgb(46, 204, 113),
                                    ">= 2.00x",
                                );
                            });

                            egui::ScrollArea::horizontal().max_height(38.0).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if self.crash.historique.is_empty() {
                                        ui.label("Aucune manche terminee");
                                        return;
                                    }

                                    for x in self.crash.historique.iter().rev().take(12) {
                                        let valeur = format_multiplicateur_historique(*x);
                                        let (stroke, texte) = if *x >= 2.0 {
                                            (
                                                egui::Color32::from_rgb(102, 255, 163),
                                                egui::RichText::new(valeur)
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(225, 255, 238)),
                                            )
                                        } else {
                                            (
                                                egui::Color32::from_rgb(255, 142, 142),
                                                egui::RichText::new(valeur)
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(255, 232, 232)),
                                            )
                                        };

                                        let badge = crash_button_style(
                                            texte,
                                            stroke,
                                            egui::vec2(84.0, 30.0),
                                        );
                                        let _ = ui.add(badge);
                                    }
                                });
                            });
                        });
                    },
                );
            },
        );

        ui.add_space(6.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), controls_fixed_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Gestion de la partie").strong().size(16.0));
                    ui.add_space(6.0);

                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            ui.label(egui::RichText::new("Mise").strong());
                            ui.add_sized(
                                [ui.available_width().max(170.0), 32.0],
                                egui::DragValue::new(&mut self.crash_mise_input)
                                    .range(0.01..=1_000_000.0)
                                    .speed(1.0)
                                    .max_decimals(2),
                            );
                            ui.add_space(8.0);
                            ui.checkbox(&mut self.crash_auto_actif, "Auto cash out");
                            ui.add_enabled_ui(self.crash_auto_actif, |ui| {
                                ui.add_sized(
                                    [ui.available_width().max(170.0), 32.0],
                                    egui::DragValue::new(&mut self.crash_auto_cashout)
                                        .range(1.01..=100.0)
                                        .speed(0.05)
                                        .max_decimals(2)
                                        .suffix("x"),
                                );
                            });
                        });

                        cols[1].vertical(|ui| {
                            let w = ui.available_width().max(210.0);

                            let btn_lancer = crash_button_style(
                                egui::RichText::new("Lancer le vol")
                                    .strong()
                                    .size(18.0)
                                    .color(egui::Color32::WHITE),
                                egui::Color32::from_rgb(93, 173, 226),
                                egui::vec2(w, 42.0),
                            )
                            .fill(egui::Color32::from_rgb(19, 31, 45));
                            if ui
                                .add_enabled(!self.crash.manche_en_cours(), btn_lancer)
                                .clicked()
                            {
                                self.crash_ui_erreur.clear();
                                if self.crash_mise_input > self.crash_solde {
                                    self.crash_ui_erreur = format!(
                                        "Solde insuffisant: balance {:.2}, mise {:.2}.",
                                        self.crash_solde, self.crash_mise_input
                                    );
                                } else if let Err(e) = self.crash.lancer_tour(self.crash_mise_input) {
                                    self.crash_ui_erreur = e;
                                } else {
                                    self.crash_solde -= self.crash_mise_input;
                                }
                            }

                            ui.add_space(8.0);
                            let cash_pulse = if self.crash.est_en_vol() {
                                0.45 + 0.55 * (anim_t * 8.0).sin().abs()
                            } else {
                                0.0
                            };
                            let btn_cashout = crash_button_style(
                                egui::RichText::new("CASH OUT")
                                    .strong()
                                    .size(20.0 + 2.5 * cash_pulse)
                                    .color(egui::Color32::WHITE),
                                egui::Color32::from_rgb(
                                    130,
                                    (220.0 + 35.0 * cash_pulse) as u8,
                                    170,
                                ),
                                egui::vec2(w, 50.0),
                            )
                            .fill(egui::Color32::from_rgb(19, 31, 45))
                            .stroke(egui::Stroke::new(
                                1.5 + 1.5 * cash_pulse,
                                egui::Color32::from_rgb(183, 255, 214),
                            ))
                            .min_size(egui::vec2(w, 50.0));
                            if ui.add_enabled(self.crash.est_en_vol(), btn_cashout).clicked() {
                                match self.crash.encaisser() {
                                    Ok(p) => {
                                        self.crash_solde += p;
                                        self.crash_ui_erreur.clear();
                                    }
                                    Err(e) => {
                                        self.crash_ui_erreur = e;
                                    }
                                }
                            }
                        });
                    });
                });
            },
        );
    }

    // ─── Mines ──────────────────────────────────────────────────────────

    fn ui_mines(&mut self, ui: &mut egui::Ui) {
        let mut retour_menu = false;
        ui.horizontal(|ui| {
            if ui.button("<- Retour menu").clicked() {
                retour_menu = true;
            }
            ui.separator();
            ui.heading("Mines Royale");
            ui.separator();
            ui.label(
                egui::RichText::new("TABLE VIP")
                    .strong()
                    .color(egui::Color32::from_rgb(247, 211, 88)),
            );
        });
        if retour_menu {
            self.ecran = EcranCasino::Menu;
            self.mines = None;
            self.mines_paiement_applique = true;
            return;
        }

        if self.mines.is_none() {
            ui.separator();
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                ui_mines_indicateur(
                    ui,
                    "BALANCE",
                    format!("{:.2}", self.mines_solde),
                    egui::Color32::from_rgb(247, 211, 88),
                );
                ui_mines_indicateur(
                    ui,
                    "MISE SELECTIONNEE",
                    format!("{:.2}", self.mines_mise_input),
                    egui::Color32::from_rgb(93, 173, 226),
                );
                ui_mines_indicateur(
                    ui,
                    "NIVEAU DE RISQUE",
                    format!("{} mines", self.mines_nb_mines),
                    egui::Color32::from_rgb(231, 76, 60),
                );
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading("Configuration de la table");
                ui.add_space(6.0);

                ui.add(
                    egui::Slider::new(&mut self.mines_nb_mines, 1..=24)
                        .text("Nombre de mines"),
                );
                ui.horizontal(|ui| {
                    ui.label("Mise:");
                    ui.add(
                        egui::DragValue::new(&mut self.mines_mise_input)
                            .range(0.01..=100_000.0)
                            .speed(1.0)
                            .max_decimals(2),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Graine client:");
                    ui.text_edit_singleline(&mut self.mines_graine_client);
                });
                ui.label(format!("Nonce: {}", self.mines_nonce));
            });

            if !self.mines_ui_erreur.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(
                    egui::Color32::from_rgb(231, 76, 60),
                    egui::RichText::new(&self.mines_ui_erreur).strong(),
                );
            }

            ui.add_space(12.0);
            let btn = egui::Button::new(
                egui::RichText::new("LANCER LA SESSION")
                    .color(egui::Color32::from_rgb(24, 20, 9))
                    .size(24.0)
                    .strong(),
            )
            .fill(egui::Color32::from_rgb(247, 196, 84))
            .min_size(egui::vec2(340.0, 58.0));

            if ui.add(btn).clicked() {
                self.mines_ui_erreur.clear();

                if self.mines_mise_input > self.mines_solde {
                    self.mines_ui_erreur = format!(
                        "Solde insuffisant: balance {:.2}, mise {:.2}.",
                        self.mines_solde, self.mines_mise_input
                    );
                    return;
                }

                // Générer une graine serveur aléatoire
                let graine_serveur: String = (0..32)
                    .map(|_| format!("{:02x}", rand::random::<u8>()))
                    .collect();
                match JeuMines::nouveau(
                    self.mines_nb_mines,
                    self.mines_mise_input,
                    self.mines_graine_client.clone(),
                    graine_serveur,
                    self.mines_nonce,
                ) {
                    Ok(jeu) => {
                        self.mines_solde -= self.mines_mise_input;
                        self.mines_paiement_applique = false;
                        self.mines = Some(jeu);
                    }
                    Err(e) => {
                        self.mines_ui_erreur = e;
                    }
                }
            }
            return;
        }

        let mut est_termine = false;
        let mut fermer = false;

        {
            let jeu = self.mines.as_mut().unwrap();
            let actif = jeu.etat == EtatMines::Actif;
            let paiement_potentiel = jeu.mise * jeu.multiplicateur;
            let total_sures = (25_u8.saturating_sub(jeu.nb_mines)).max(1);
            let progression = (jeu.cases_revelees as f32 / total_sures as f32).clamp(0.0, 1.0);

            ui.separator();
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                ui_mines_indicateur(
                    ui,
                    "BALANCE",
                    format!("{:.2}", self.mines_solde),
                    egui::Color32::from_rgb(247, 211, 88),
                );
                ui_mines_indicateur(
                    ui,
                    "MISE ACTIVE",
                    format!("{:.2}", jeu.mise),
                    egui::Color32::from_rgb(93, 173, 226),
                );
                ui_mines_indicateur(
                    ui,
                    "MULTIPLICATEUR",
                    format!("{:.4}x", jeu.multiplicateur),
                    egui::Color32::from_rgb(46, 204, 113),
                );
                ui_mines_indicateur(
                    ui,
                    "CASH OUT",
                    format!("{:.2}", paiement_potentiel),
                    egui::Color32::from_rgb(245, 176, 65),
                );
                ui_mines_indicateur(
                    ui,
                    "CASES OUVERTES",
                    format!("{}/{}", jeu.cases_revelees, total_sures),
                    egui::Color32::from_rgb(171, 235, 198),
                );
            });

            ui.add_space(6.0);
            ui.add(
                egui::ProgressBar::new(progression)
                    .desired_width(ui.available_width())
                    .fill(egui::Color32::from_rgb(26, 148, 95))
                    .text(format!(
                        "Progression gemmes: {} / {}",
                        jeu.cases_revelees, total_sures
                    )),
            );

            let couleur_message = match jeu.etat {
                EtatMines::Perdu => egui::Color32::from_rgb(231, 76, 60),
                EtatMines::Gagne(_) => egui::Color32::from_rgb(46, 204, 113),
                _ => egui::Color32::from_rgb(226, 232, 240),
            };
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&jeu.message)
                    .size(19.0)
                    .strong()
                    .color(couleur_message),
            );

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("Hash serveur:")
                        .color(egui::Color32::from_rgb(148, 163, 184))
                        .strong(),
                );
                ui.monospace(format!("{}...", &jeu.hash_graine_serveur[..16]));
            });

            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                let btn_cashout = egui::Button::new(
                    egui::RichText::new(format!("CASH OUT {:.2}", paiement_potentiel))
                        .size(30.0)
                        .strong()
                        .color(egui::Color32::from_rgb(24, 20, 9)),
                )
                .fill(egui::Color32::from_rgb(247, 196, 84))
                .min_size(egui::vec2(420.0, 68.0));

                let peut_encaisser = actif && jeu.cases_revelees > 0;
                if ui.add_enabled(peut_encaisser, btn_cashout).clicked() {
                    let _ = jeu.encaisser();
                }
                if !peut_encaisser {
                    ui.label(
                        egui::RichText::new("Ouvre au moins une gemme pour débloquer le cash out.")
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );
                }

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("AUTOPLAY")
                            .strong()
                            .color(egui::Color32::from_rgb(189, 195, 199)),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.mines_autoplay_count, 1..=20)
                                .text("Cases"),
                        );
                        let btn_auto = egui::Button::new(
                            egui::RichText::new("LANCER").color(egui::Color32::WHITE).strong(),
                        )
                        .fill(egui::Color32::from_rgb(52, 73, 94));
                        if ui.add_enabled(actif, btn_auto).clicked() {
                            let n = self.mines_autoplay_count;
                            jeu.autoplay(n);
                        }
                    });
                });
            });

            ui.add_space(12.0);
            let cell_size = 80.0;
            let gap = 8.0;
            let grid_size = cell_size * 5.0 + gap * 4.0;
            let grid_rect_size = egui::vec2(grid_size, grid_size);
            ui.vertical_centered(|ui| {
                let (grid_rect, _) =
                    ui.allocate_exact_size(grid_rect_size, egui::Sense::hover());

                let mut case_cliquee: Option<(usize, usize)> = None;

                for ligne in 0..5 {
                    for col in 0..5 {
                        let x = grid_rect.left() + col as f32 * (cell_size + gap);
                        let y = grid_rect.top() + ligne as f32 * (cell_size + gap);
                        let cell_rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(cell_size, cell_size),
                        );

                        let case = jeu.grille[ligne][col];
                        dessiner_case_mine(ui, cell_rect, case, actif, ligne, col);

                        // Détection clic
                        if actif && case == CaseMine::Cachee {
                            let resp = ui.allocate_rect(cell_rect, egui::Sense::click());
                            if resp.clicked() {
                                case_cliquee = Some((ligne, col));
                            }
                        }
                    }
                }

                // Appliquer le clic
                if let Some((l, c)) = case_cliquee {
                    let _ = jeu.reveler(l, c);
                }
            });

            est_termine = jeu.est_termine();
        }

        if est_termine && !self.mines_paiement_applique {
            if let Some(jeu) = self.mines.as_ref() {
                if let EtatMines::Gagne(p) = jeu.etat {
                    self.mines_solde += p;
                }
            }
            self.mines_paiement_applique = true;
        }

        if est_termine {
            let jeu = self.mines.as_ref().unwrap();
            ui.add_space(8.0);
            ui.separator();

            match jeu.etat {
                EtatMines::Gagne(p) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(46, 204, 113),
                        egui::RichText::new(format!("GAIN CONFIRME: {:.2}", p))
                            .size(22.0)
                            .strong(),
                    );
                }
                EtatMines::Perdu => {
                    ui.colored_label(
                        egui::Color32::from_rgb(231, 76, 60),
                        egui::RichText::new(format!("PERDU: mise {:.2}", jeu.mise))
                            .size(22.0)
                            .strong(),
                    );
                }
                _ => {}
            }
            ui.colored_label(
                egui::Color32::from_rgb(247, 211, 88),
                egui::RichText::new(format!("Balance actuelle: {:.2}", self.mines_solde))
                    .size(20.0)
                    .strong(),
            );

            // Vérification d'équité
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Graine serveur:");
                ui.monospace(&jeu.graine_serveur);
            });
            ui.horizontal(|ui| {
                ui.label("Verification equite:");
                if jeu.verifier_equite() {
                    ui.colored_label(egui::Color32::from_rgb(46, 204, 113), "Verifiee");
                } else {
                    ui.colored_label(egui::Color32::RED, "Echec");
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Nouvelle partie")
                                .size(18.0)
                                .strong()
                                .color(egui::Color32::from_rgb(24, 20, 9)),
                        )
                        .fill(egui::Color32::from_rgb(247, 196, 84))
                        .min_size(egui::vec2(210.0, 44.0)),
                    )
                    .clicked()
                {
                    self.mines_nonce += 1;
                    fermer = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Retour menu")
                                .size(17.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(52, 73, 94))
                        .min_size(egui::vec2(180.0, 44.0)),
                    )
                    .clicked()
                {
                    self.ecran = EcranCasino::Menu;
                    fermer = true;
                }
            });
        }

        if fermer {
            self.mines = None;
            self.mines_paiement_applique = true;
        }
    }
}

fn crash_button_style(
    texte: egui::RichText,
    accent: egui::Color32,
    taille: egui::Vec2,
) -> egui::Button<'static> {
    egui::Button::new(texte)
        .fill(egui::Color32::from_rgb(19, 31, 45))
        .stroke(egui::Stroke::new(1.2, accent))
        .min_size(taille)
}

fn ui_crash_indicateur(ui: &mut egui::Ui, titre: &str, valeur: String, accent: egui::Color32) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(19, 31, 45))
        .stroke(egui::Stroke::new(1.0, accent))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(150.0, 58.0));
            ui.label(
                egui::RichText::new(titre)
                    .size(11.0)
                    .strong()
                    .color(egui::Color32::from_rgb(160, 174, 192)),
            );
            ui.add_space(1.0);
            ui.label(egui::RichText::new(valeur).size(21.0).strong().color(accent));
        });
}

fn dessiner_graphique_crash(ui: &mut egui::Ui, rect: egui::Rect, jeu: &JeuCrash) {
    let time = ui.ctx().input(|i| i.time) as f32;
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 14.0, egui::Color32::from_rgb(9, 18, 31));
    painter.rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(1.4, egui::Color32::from_rgb(67, 100, 133)),
        egui::StrokeKind::Outside,
    );

    let zone = rect.shrink2(egui::vec2(18.0, 18.0));

    // Etoiles animées pour l'ambiance (plus denses)
    for i in 0..56 {
        let x_phase = ((time * 0.04 + i as f32 * 0.113) % 1.0).abs();
        let y_phase = ((i as f32 * 0.251 + 0.19) % 1.0).abs();
        let alpha = (45.0 + ((time * 3.0 + i as f32).sin() * 40.0 + 40.0)) as u8;
        let rayon = 1.1 + ((i as f32 * 0.3).sin().abs() * 0.9);
        painter.circle_filled(
            egui::pos2(
                zone.left() + x_phase * zone.width(),
                zone.top() + y_phase * zone.height(),
            ),
            rayon,
            egui::Color32::from_rgba_premultiplied(187, 208, 255, alpha),
        );
    }

    painter.line_segment(
        [
            egui::pos2(zone.left(), zone.bottom()),
            egui::pos2(zone.right(), zone.bottom()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(93, 109, 126)),
    );
    painter.line_segment(
        [
            egui::pos2(zone.left(), zone.bottom()),
            egui::pos2(zone.left(), zone.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(93, 109, 126)),
    );

    // Départ à gauche puis course sur toute la largeur.
    let x_depart = zone.left() + 10.0;
    let largeur_course = (zone.right() - x_depart - 8.0).max(12.0);
    let mult_pour_trace = if jeu.manche_en_cours() {
        jeu.multiplicateur_vol()
    } else {
        1.0
    };
    let point_fin = jeu
        .point_crash_revele()
        .unwrap_or(jeu.multiplicateur_vol().max(2.0))
        .max(2.0);
    let ymax = point_fin.max(5.0) as f32;
    let progression = ((mult_pour_trace as f32 - 1.0) / (ymax - 1.0)).clamp(0.0, 1.0);
    let x_courant = x_depart + progression * largeur_course;

    let mut points: Vec<egui::Pos2> = Vec::new();
    let n = ((progression * 140.0).ceil() as usize).max(2);
    for i in 0..n {
        let t_local = i as f32 / (n - 1) as f32;
        let t_global = progression * t_local;
        let x = x_depart + t_local * (x_courant - x_depart);
        let wobble = if jeu.est_en_vol() {
            (time * 6.0 + t_global * 12.0).sin() * 0.015 * t_global
        } else {
            0.0
        };
        let y = zone.bottom() - (t_global.powf(1.6) + wobble) * zone.height() * 0.86;
        points.push(egui::pos2(x, y));
    }

    if points.len() >= 2 {
        painter.add(egui::Shape::line(
            points.clone(),
            egui::Stroke::new(7.0, egui::Color32::from_rgba_premultiplied(46, 204, 113, 45)),
        ));
        painter.add(egui::Shape::line(
            points.clone(),
            egui::Stroke::new(2.8, egui::Color32::from_rgb(89, 238, 146)),
        ));
    }

    if let Some(fin) = points.last().copied() {
        let size = if jeu.est_en_vol() {
            let pulse = 1.0 + 0.08 * (time * 10.0).sin().abs();
            egui::vec2(84.0, 84.0) * pulse
        } else {
            egui::vec2(80.0, 80.0)
        };
        let center = fin + egui::vec2(0.0, -6.0);
        let clamped_center = egui::pos2(
            center
                .x
                .clamp(zone.left() + size.x * 0.5 + 2.0, zone.right() - size.x * 0.5 - 2.0),
            center
                .y
                .clamp(zone.top() + size.y * 0.5 + 2.0, zone.bottom() - size.y * 0.5 - 2.0),
        );
        let img_rect = egui::Rect::from_center_size(clamped_center, size);
        ui.put(
            img_rect,
            egui::Image::new(egui::include_image!("../../avion.png"))
                .fit_to_exact_size(img_rect.size()),
        );
    }

    if let Some(c) = jeu.point_crash_revele() {
        let ratio = ((c as f32 - 1.0) / (ymax - 1.0)).clamp(0.0, 1.0);
        let x = x_depart + ratio * largeur_course;
        painter.line_segment(
            [egui::pos2(x, zone.top()), egui::pos2(x, zone.bottom())],
            egui::Stroke::new(1.3, egui::Color32::from_rgb(231, 76, 60)),
        );
        painter.text(
            egui::pos2(x + 6.0, zone.top() + 8.0),
            egui::Align2::LEFT_TOP,
            format!("BOOM {:.2}x", c),
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(231, 76, 60),
        );
    }

    painter.text(
        zone.left_top() + egui::vec2(6.0, 2.0),
        egui::Align2::LEFT_TOP,
        "x",
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(180, 190, 200),
    );
    painter.text(
        egui::pos2(x_depart, zone.bottom() + 2.0),
        egui::Align2::CENTER_TOP,
        "depart",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(160, 174, 192),
    );
    painter.text(
        zone.right_bottom() + egui::vec2(-8.0, -2.0),
        egui::Align2::RIGHT_BOTTOM,
        "temps",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(180, 190, 200),
    );
}

fn format_multiplicateur_historique(mult: f64) -> String {
    let mut base = format!("{:.4}", mult.max(0.0));
    while base.ends_with('0') {
        base.pop();
    }
    if base.ends_with('.') {
        base.push('0');
    }
    format!("{}x", base.replace('.', ","))
}

fn ui_mines_indicateur(ui: &mut egui::Ui, titre: &str, valeur: String, accent: egui::Color32) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(19, 31, 45))
        .stroke(egui::Stroke::new(1.0, accent))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(180.0, 68.0));
            ui.label(
                egui::RichText::new(titre)
                    .size(12.0)
                    .strong()
                    .color(egui::Color32::from_rgb(160, 174, 192)),
            );
            ui.add_space(2.0);
            ui.label(egui::RichText::new(valeur).size(25.0).strong().color(accent));
        });
}

fn dessiner_case_mine(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    case: CaseMine,
    actif: bool,
    ligne: usize,
    col: usize,
) {
    let mut render_rect = rect;

    // Animation au survol (agrandissement)
    if actif && case == CaseMine::Cachee {
        let is_hovered = ui.rect_contains_pointer(rect);
        // Utilisation du temps pour lisser l'animation du scale
        let hover_factor = ui.ctx().animate_bool_with_time(
            ui.id().with(ligne).with(col),
            is_hovered,
            0.15,
        );
        let scale = 1.0 + 0.05 * hover_factor; // grossit de 5%
        render_rect = egui::Rect::from_center_size(rect.center(), rect.size() * scale);
    }

    let painter = ui.painter_at(render_rect);

    match case {
        CaseMine::Cachee => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(34, 47, 62));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(87, 101, 116)),
                egui::StrokeKind::Outside,
            );
            // Effet d'éclat interne léger
            painter.rect_filled(render_rect.shrink(4.0), 8.0, egui::Color32::from_rgb(40, 55, 71));
        }
        CaseMine::Revelee => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(11, 83, 69));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(3.0, egui::Color32::from_rgb(46, 204, 113)),
                egui::StrokeKind::Outside,
            );

            // Image Diamant
            let img_rect = render_rect.shrink(10.0);
            ui.put(
                img_rect,
                egui::Image::new(egui::include_image!("../../diamond.png"))
                    .fit_to_exact_size(img_rect.size()),
            );
        }
        CaseMine::MineRevelee => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(146, 43, 33));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(3.0, egui::Color32::from_rgb(231, 76, 60)),
                egui::StrokeKind::Outside,
            );

            // Image Bombe
            let img_rect = render_rect.shrink(10.0);
            ui.put(
                img_rect,
                egui::Image::new(egui::include_image!("../../mines.png"))
                    .fit_to_exact_size(img_rect.size()),
            );
        }
        CaseMine::MineMontree => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(60, 20, 20));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(150, 50, 50)),
                egui::StrokeKind::Outside,
            );

            // Image Bombe transparente
            let img_rect = render_rect.shrink(12.0);
            let mut img = egui::Image::new(egui::include_image!("../../mines.png"))
                .fit_to_exact_size(img_rect.size());
            img = img.tint(egui::Color32::from_white_alpha(100));
            ui.put(img_rect, img);
        }
    }
}

fn dessiner_table_blackjack(ui: &mut egui::Ui, rect: egui::Rect, jeu: &JeuBlackjack) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 18.0, egui::Color32::from_rgb(12, 28, 24));
    let table = rect.shrink2(egui::vec2(18.0, 12.0));
    painter.rect_filled(table, 120.0, egui::Color32::from_rgb(18, 96, 66));
    painter.rect_stroke(
        table,
        120.0,
        egui::Stroke::new(4.0, egui::Color32::from_rgb(132, 85, 50)),
        egui::StrokeKind::Outside,
    );

    let c = table.center();
    let dealer_zone =
        egui::Rect::from_center_size(egui::pos2(c.x, table.top() + 34.0), egui::vec2(420.0, 52.0));
    dessiner_zone_label(
        &painter,
        dealer_zone,
        &format!("Croupier | Score: {}", jeu.score_croupier_visible()),
    );

    let dealer_y = table.top() + 82.0;
    for (i, card) in jeu.main_croupier.iter().enumerate() {
        let card_rect = egui::Rect::from_min_size(
            egui::pos2(c.x - 150.0 + i as f32 * 74.0, dealer_y),
            egui::vec2(62.0, 90.0),
        );
        let cachee = jeu.croupier_cachee() && i == 0;
        if cachee {
            dessiner_carte(ui, &painter, card_rect, None, false);
        } else {
            dessiner_carte(ui, &painter, card_rect, Some(card), true);
        }
    }

    let actifs: Vec<usize> = jeu
        .joueurs
        .iter()
        .enumerate()
        .filter_map(|(i, j)| if j.actif() { Some(i) } else { None })
        .collect();
    let nb = actifs.len().max(1) as f32;
    let zone_y = table.bottom() - 28.0;
    let x_start = table.left() + 120.0;
    let x_end = table.right() - 120.0;
    let step = if nb > 1.0 {
        (x_end - x_start) / (nb - 1.0)
    } else {
        0.0
    };

    for (pos, idx) in actifs.iter().enumerate() {
        let j = &jeu.joueurs[*idx];
        let x_center = x_start + step * pos as f32;
        let zone_w = if nb >= 5.0 { 180.0 } else { 220.0 };
        let zone = egui::Rect::from_center_size(egui::pos2(x_center, zone_y), egui::vec2(zone_w, 44.0));
        let titre = if *idx == 0 {
            format!("Toi | {}", jeu.score_joueur(*idx))
        } else {
            format!("{} | {}", j.nom, jeu.score_joueur(*idx))
        };
        dessiner_zone_label(&painter, zone, &titre);

        let cards_y = zone.top() - 118.0;
        for (k, card) in j.main.iter().enumerate() {
            let card_rect = egui::Rect::from_min_size(
                egui::pos2(x_center - 58.0 + k as f32 * 38.0, cards_y),
                egui::vec2(56.0, 82.0),
            );
            dessiner_carte(ui, &painter, card_rect, Some(card), true);
        }
        if j.main.is_empty() {
            let card_rect =
                egui::Rect::from_min_size(egui::pos2(x_center - 28.0, cards_y), egui::vec2(56.0, 82.0));
            dessiner_carte(ui, &painter, card_rect, None, false);
        }
    }

    let pot_rect = egui::Rect::from_center_size(egui::pos2(c.x, c.y + 58.0), egui::vec2(210.0, 48.0));
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
        format!("MISE REF {}", jeu.mise_reference),
        egui::FontId::proportional(20.0),
        egui::Color32::from_rgb(238, 220, 151),
    );
    dessiner_jetons(
        &painter,
        egui::pos2(pot_rect.left() - 34.0, pot_rect.center().y),
        3,
    );
    dessiner_jetons(
        &painter,
        egui::pos2(pot_rect.right() + 24.0, pot_rect.center().y),
        3,
    );
}

fn dessiner_zone_label(painter: &egui::Painter, rect: egui::Rect, texte: &str) {
    painter.rect_filled(
        rect,
        12.0,
        egui::Color32::from_rgba_premultiplied(5, 20, 16, 170),
    );
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
        egui::FontId::proportional(20.0),
        egui::Color32::from_rgb(220, 232, 227),
    );
}

fn dessiner_table_solo(ui: &mut egui::Ui, rect: egui::Rect, game: &PokerGuiGame) {
    let painter = ui.painter_at(rect);
    let bg = egui::Color32::from_rgb(13, 30, 24);
    painter.rect_filled(rect, 18.0, bg);

    let table_rect = rect.shrink2(egui::vec2(24.0, 18.0));
    painter.rect_filled(table_rect, 120.0, egui::Color32::from_rgb(18, 92, 64));
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

fn dessiner_table_online(ui: &mut egui::Ui, rect: egui::Rect, state: &OnlinePokerState) {
    let painter = ui.painter_at(rect);
    let bg = egui::Color32::from_rgb(13, 30, 24);
    painter.rect_filled(rect, 18.0, bg);

    let table_rect = rect.shrink2(egui::vec2(24.0, 18.0));
    painter.rect_filled(table_rect, 120.0, egui::Color32::from_rgb(18, 92, 64));
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
        if state.en_attente_action {
            "Toi - ton tour"
        } else {
            "Toi"
        },
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

fn dessiner_joueur_zone(
    painter: &egui::Painter,
    rect: egui::Rect,
    nom: &str,
    jetons: u32,
    mise_tour: u32,
) {
    painter.rect_filled(
        rect,
        12.0,
        egui::Color32::from_rgba_premultiplied(5, 20, 16, 170),
    );
    painter.rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(79, 124, 101)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{}  |  Stack: {}  |  Mise: {}", nom, jetons, mise_tour),
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(220, 232, 227),
    );
}

fn dessiner_carte(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    card: Option<&Carte>,
    face_up: bool,
) {
    if face_up {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(249, 249, 245));
        painter.rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(74, 74, 80)),
            egui::StrokeKind::Outside,
        );
        if let Some(c) = card {
            let image_rect = rect.shrink(3.0);
            ui.put(
                image_rect,
                egui::Image::new(c.image_url_api()).fit_to_exact_size(image_rect.size()),
            );
            let txt = c.to_string();
            let red = txt.ends_with('C') || txt.ends_with('D');
            painter.text(
                rect.left_top() + egui::vec2(4.0, 3.0),
                egui::Align2::LEFT_TOP,
                txt,
                egui::FontId::proportional(13.0),
                if red {
                    egui::Color32::from_rgb(191, 39, 45)
                } else {
                    egui::Color32::from_rgb(22, 24, 28)
                },
            );
        }
    } else {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(24, 47, 93));
        painter.rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(112, 148, 220)),
            egui::StrokeKind::Outside,
        );
        painter.rect_filled(rect.shrink(8.0), 6.0, egui::Color32::from_rgb(38, 62, 111));
    }
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
