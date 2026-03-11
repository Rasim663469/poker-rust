use crate::core::cards::{Carte, Paquet};
use crate::core::player::Joueur;
use crate::games::blackjack::engine::{EtatBlackjack, JeuBlackjack};
use crate::games::poker::engine::evaluer_holdem_pour_gui;
use crate::network::protocol::{ActionJoueur, MessageClient, MessageServeur};
use crate::network::{recv_json, send_json};
use crate::games::slotmachine::SlotMachine;
use crate::interface::roulette_gui::{RouletteGuiState, ui_roulette};
use eframe::egui;
use rand::Rng;
use std::cmp::Ordering;
use std::sync::mpsc;
use std::thread;
use tokio::net::TcpStream;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EcranCasino {
    Menu,
    Poker,
    Blackjack,
    SlotMachine,
    Roulette,
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
    slot_symbols: [usize; 3],
    slot_result: String,
    roulette_state: RouletteGuiState,
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
            slot_symbols: [0, 1, 2],
            slot_result: String::new(),
            roulette_state: RouletteGuiState::default(),
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

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Casino Rust");
                ui.separator();
                ui.label(match self.ecran {
                    EcranCasino::Menu => "Menu",
                    EcranCasino::Poker => "Poker jouable en GUI",
                    EcranCasino::Blackjack => "Blackjack jouable en GUI",
                    EcranCasino::SlotMachine => "Machine à sous",
                    EcranCasino::Roulette => "Roulette",
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.ecran {
            EcranCasino::Menu => self.ui_menu(ui),
            EcranCasino::Poker => self.ui_poker(ui),
            EcranCasino::Blackjack => self.ui_blackjack(ui),
            EcranCasino::SlotMachine => self.ui_slot_machine(ui),
            EcranCasino::Roulette => {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.heading("Roulette");
                    ui.add_space(20.0);
                    ui_roulette(&mut self.roulette_state, ui);
                    ui.add_space(10.0);
                    if ui.button("<- Retour menu").clicked() {
                        self.ecran = EcranCasino::Menu;
                    }
                });
            }
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
        if ui.button("Machine à sous").clicked() {
            self.ecran = EcranCasino::SlotMachine;
        }
        if ui.button("Roulette").clicked() {
            self.ecran = EcranCasino::Roulette;
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

    fn ui_slot_machine(&mut self, ui: &mut egui::Ui) {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.heading("Machine à sous");
            ui.add_space(20.0);
            let highlight = self.slot_symbols[0] == self.slot_symbols[1] && self.slot_symbols[1] == self.slot_symbols[2];
            dessiner_slot_machine(ui, &self.slot_symbols, highlight);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(750.0);
                if ui.add(egui::Button::new("Lancer !").min_size(egui::vec2(100.0, 40.0))).clicked() {
                    let result = SlotMachine::spin();
                    self.slot_symbols = result.symbols;
                    self.slot_result = if result.win {
                        "Jackpot !".to_string()
                    } else {
                        "Perdu...".to_string()
                    };
                }
            });
            ui.add_space(10.0);
            if highlight {
                ui.colored_label(egui::Color32::from_rgb(255, 215, 0), &self.slot_result);
            } else {
                ui.label(&self.slot_result);
            }
            if ui.button("<- Retour menu").clicked() {
                self.ecran = EcranCasino::Menu;
            }
        });
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

fn dessiner_slot_machine(ui: &mut egui::Ui, symbols: &[usize; 3], highlight: bool) {
    static SYMBOLS: [&str; 4] = ["🍒", "🍋", "🔔", "7"];
    // Agrandir la zone pour inclure le levier
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(400.0, 140.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    // Fond de la machine
    painter.rect_filled(rect, 16.0, egui::Color32::from_rgb(40, 40, 40));
    // Cadre
    painter.rect_stroke(rect, 16.0, egui::Stroke::new(3.0, egui::Color32::from_rgb(200, 180, 60)), egui::StrokeKind::Outside);
    // Rouleaux
    for i in 0..3 {
        let x = rect.left() + 40.0 + i as f32 * 86.0;
        let y = rect.top() + 30.0;
        let slot_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(60.0, 80.0));
        let color = if highlight { egui::Color32::from_rgb(255, 220, 80) } else { egui::Color32::from_rgb(230, 230, 230) };
        painter.rect_filled(slot_rect, 12.0, color);
        painter.rect_stroke(slot_rect, 8.0, egui::Stroke::new(2.0, egui::Color32::GRAY), egui::StrokeKind::Outside);
        painter.text(
            slot_rect.center(),
            egui::Align2::CENTER_CENTER,
            SYMBOLS[symbols[i]],
            egui::FontId::proportional(48.0),
            egui::Color32::BLACK,
        );
    }
    // Levier à l'intérieur du cadre, à droite
    let levier_x = rect.right() - 30.0;
    let levier_y = rect.center().y;
    let levier_top = egui::pos2(levier_x, levier_y - 30.0);
    let levier_bottom = egui::pos2(levier_x, levier_y + 30.0);
    painter.line_segment([
        levier_top,
        levier_bottom
    ], egui::Stroke::new(6.0, egui::Color32::DARK_GRAY));
    painter.circle_filled(levier_top, 10.0, egui::Color32::from_rgb(220, 0, 0));
    painter.circle_filled(levier_bottom, 12.0, egui::Color32::from_rgb(180, 180, 180));
}
