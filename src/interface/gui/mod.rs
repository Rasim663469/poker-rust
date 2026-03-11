use crate::games::blackjack::engine::JeuBlackjack;
use eframe::egui;
use std::sync::mpsc;

mod blackjack;
mod draw;
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
}

impl Default for CasinoApp {
    fn default() -> Self {
        Self {
            ecran: EcranCasino::Menu,
            poker_vue: PokerVue::Choix,
            banque_joueur: 1000,
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
                    EcranCasino::SlotMachine => "Machine a sous",
                });
                ui.separator();
                ui.label(format!("Banque : {} €", self.banque_joueur));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.ecran {
            EcranCasino::Menu => self.ui_menu(ui),
            EcranCasino::Poker => self.ui_poker(ui),
            EcranCasino::Blackjack => self.ui_blackjack(ui),
            EcranCasino::SlotMachine => self.ui_slot_machine(ui),
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
    }
}
