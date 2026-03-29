use crate::games::blackjack::engine::JeuBlackjack;
use crate::games::hilo::AceMode;
use crate::games::roulette::RouletteResult;
use eframe::egui;
use std::sync::mpsc;

mod blackjack;
mod draw;
mod hilo;
mod login;
mod poker;
mod poker_online;
mod roulette;
mod slotmachine;

use self::login::LoginState;
use self::poker::PokerGuiGame;
use self::poker_online::OnlinePokerState;
use self::roulette::RouletteBetUI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EcranCasino {
    Login,
    Menu,
    Poker,
    Blackjack,
    SlotMachine,
    HiLo,
    Roulette,
    Depot,
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

pub struct CasinoApp {
    ecran: EcranCasino,
    joueur_pseudo: String,
    joueur_db_id: Option<i32>,
    login: LoginState,
    poker_vue: PokerVue,
    banque_joueur: u32,
    jetons_depart: u32,
    small_blind: u32,
    big_blind: u32,
    poker: Option<PokerGuiGame>,
    poker_online: OnlinePokerState,
    tx_online: Option<mpsc::Sender<crate::network::protocol::ActionJoueur>>,
    rx_online: Option<mpsc::Receiver<crate::network::protocol::MessageServeur>>,
    blackjack: Option<JeuBlackjack>,
    bj_nb_joueurs: u8,
    bj_jetons_depart: u32,
    bj_mise_input: u32,
    slot_symbols: [usize; 3],
    slot_result: String,
    slot_mise: u32,
    hilo: Option<crate::games::hilo::HiLoGame>,
    hilo_jetons_depart: u32,
    hilo_mise_input: u32,
    hilo_allow_equal: bool,
    hilo_ace_mode: AceMode,
    hilo_payout_win: u32,
    hilo_payout_equal: u32,
    hilo_min_bet: u32,
    hilo_max_bet: u32,
    hilo_last_outcome: Option<crate::games::hilo::HiLoOutcome>,
    hilo_reveal_at: Option<std::time::Instant>,
    roulette_bet: RouletteBetUI,
    roulette_mise: u32,
    roulette_last_result: Option<RouletteResult>,
    roulette_anim: Option<roulette::RouletteAnim>,
    depot_input: u32,
}

impl Default for CasinoApp {
    fn default() -> Self {
        Self {
            ecran: EcranCasino::Login,
            joueur_pseudo: String::new(),
            joueur_db_id: None,
            login: LoginState::default(),
            poker_vue: PokerVue::Choix,
            banque_joueur: 0,
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
            slot_mise: 10,
            hilo: None,
            hilo_jetons_depart: 500,
            hilo_mise_input: 10,
            hilo_allow_equal: false,
            hilo_ace_mode: AceMode::High,
            hilo_payout_win: 1,
            hilo_payout_equal: 5,
            hilo_min_bet: 1,
            hilo_max_bet: 1000,
            hilo_last_outcome: None,
            hilo_reveal_at: None,
            roulette_bet: RouletteBetUI::None,
            roulette_mise: 10,
            roulette_last_result: None,
            roulette_anim: None,
            depot_input: 100,
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

        if self.ecran == EcranCasino::Login {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.ui_login(ui, ctx);
            });
            return;
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Casino Rust");
                ui.separator();
                ui.label(match self.ecran {
                    EcranCasino::Login => "",
                    EcranCasino::Menu => "Menu",
                    EcranCasino::Poker => "Poker Texas Hold'em",
                    EcranCasino::Blackjack => "Blackjack",
                    EcranCasino::SlotMachine => "Machine à sous",
                    EcranCasino::HiLo => "Hi-Lo",
                    EcranCasino::Roulette => "Roulette",
                    EcranCasino::Depot => "Dépôt",
                });
                ui.separator();
                ui.label(format!("👤 {}", self.joueur_pseudo));
                ui.separator();
                ui.label(format!("💰 {} €", self.banque_joueur));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Déconnexion").clicked() {
                        self.login = LoginState::default();
                        self.ecran = EcranCasino::Login;
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.ecran {
            EcranCasino::Login => {}
            EcranCasino::Menu => self.ui_menu(ui),
            EcranCasino::Poker => self.ui_poker(ui),
            EcranCasino::Blackjack => self.ui_blackjack(ui),
            EcranCasino::SlotMachine => self.ui_slot_machine(ui),
            EcranCasino::HiLo => self.ui_hilo(ui),
            EcranCasino::Roulette => self.ui_roulette(ui),
            EcranCasino::Depot => self.ui_depot(ui),
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
            self.poker_vue = PokerVue::Choix;
        }
        if ui.button("Blackjack").clicked() {
            self.ecran = EcranCasino::Blackjack;
        }
        if ui.button("Machine a sous").clicked() {
            self.ecran = EcranCasino::SlotMachine;
        }
        if ui.button("Hi-Lo").clicked() {
            self.ecran = EcranCasino::HiLo;
        }
        if ui.button("Roulette").clicked() {
            self.ecran = EcranCasino::Roulette;
        }
        ui.add_space(14.0);
        ui.separator();
        if ui.button("💰 Ajouter de l'argent").clicked() {
            self.depot_input = 100;
            self.ecran = EcranCasino::Depot;
        }
    }

    fn ui_depot(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("<- Retour menu").clicked() {
                self.ecran = EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Recharger la banque");
        });

        ui.add_space(20.0);
        ui.label(format!("Solde actuel : {} €", self.banque_joueur));
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label("Montant a ajouter :");
            ui.add(
                egui::DragValue::new(&mut self.depot_input)
                    .range(10..=u32::MAX)
                    .prefix("")
                    .suffix(" €")
                    .speed(10.0),
            );
        });

        ui.add_space(6.0);
        ui.label("Montant rapide :");
        ui.horizontal(|ui| {
            for montant in [50, 100, 200, 500, 1000] {
                if ui.button(format!("+{} €", montant)).clicked() {
                    self.depot_input = montant;
                }
            }
        });

        ui.add_space(14.0);
        if ui.button(format!("✅ Ajouter {} € au compte", self.depot_input)).clicked() {
            self.banque_joueur += self.depot_input;
            self.ecran = EcranCasino::Menu;
        }
    }
}
