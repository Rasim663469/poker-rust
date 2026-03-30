use crate::games::blackjack::engine::JeuBlackjack;
use crate::games::crash::engine::JeuCrash;
use crate::games::hilo::AceMode;
use crate::games::mines::engine::JeuMines;
use eframe::egui;
use std::sync::mpsc;
use std::time::Instant;

mod blackjack;
mod crash;
mod draw;
mod hilo;
mod mines;
mod poker;
mod poker_online;
mod slotmachine;

use self::poker::PokerGuiGame;
use self::poker_online::OnlinePokerState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EcranCasino {
    Menu,
    Poker,
    Blackjack,
    SlotMachine,
    HiLo,
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

pub struct CasinoApp {
    ecran: EcranCasino,
    poker_vue: PokerVue,
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
            slot_symbols: [0, 1, 2],
            slot_result: String::new(),
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
                    EcranCasino::SlotMachine => "Machine a sous",
                    EcranCasino::HiLo => "Hi-Lo",
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
            EcranCasino::SlotMachine => self.ui_slot_machine(ui),
            EcranCasino::HiLo => self.ui_hilo(ui),
            EcranCasino::Mines => self.ui_mines(ui),
            EcranCasino::Crash => self.ui_crash(ui),
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
        if ui.button("Mines").clicked() {
            self.ecran = EcranCasino::Mines;
        }
        if ui.button("Crash").clicked() {
            self.ecran = EcranCasino::Crash;
        }
    }
}
