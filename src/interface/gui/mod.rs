use crate::games::blackjack::engine::JeuBlackjack;
use crate::games::hilo::AceMode;
use crate::games::roulette::RouletteResult;
use eframe::egui;
use std::sync::mpsc;

mod assets;
mod blackjack;
mod draw;
mod hilo;
mod login;
mod poker;
mod poker_online;
mod roulette;
mod slotmachine;
mod theme;

use self::assets::GameAsset;
use self::poker::PokerGuiGame;
use self::poker_online::OnlinePokerState;
use self::theme::{apply_casino_theme, game_tile, lobby_hero, panel_frame, section_title, subpanel_frame, BG_DARK, GOLD_SOFT, TEXT_DIM};

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
    slot_anim: Option<slotmachine::SlotMachineAnim>,
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
    roulette_mise: u32,  // Slider value
    roulette_mise_en_jeu: u32,  // Actual deducted bet
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
            slot_anim: None,
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
            roulette_mise_en_jeu: 0,
            roulette_last_result: None,
            roulette_anim: None,
            depot_input: 100,
        }
    }
}

impl eframe::App for CasinoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_casino_theme(ctx);
        self.pomper_messages_online();

        if let Some(p) = &mut self.poker {
            p.bot_jouer_si_tour();
        }
        if let Some(bj) = &mut self.blackjack {
            bj.avancer_automatique();
        }

        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(12, 17, 23))
                    .inner_margin(egui::Margin::symmetric(18, 14)),
            )
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("CASINO RUST")
                        .size(28.0)
                        .strong()
                        .color(GOLD_SOFT),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(match self.ecran {
                    EcranCasino::Menu => "Menu",
                    EcranCasino::Poker => "Poker Texas Hold'em",
                    EcranCasino::Blackjack => "Blackjack",
                    EcranCasino::SlotMachine => "Machine à sous",
                    EcranCasino::HiLo => "Hi-Lo",
                    })
                    .color(TEXT_DIM),
                );
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(egui::Margin::same(18)))
            .show(ctx, |ui| match self.ecran {
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
        let bg = ui.max_rect();
        let painter = ui.painter();
        painter.circle_filled(
            egui::pos2(bg.right() - 180.0, bg.top() + 120.0),
            180.0,
            egui::Color32::from_rgba_premultiplied(186, 138, 36, 18),
        );
        painter.circle_filled(
            egui::pos2(bg.left() + 120.0, bg.bottom() - 120.0),
            160.0,
            egui::Color32::from_rgba_premultiplied(28, 126, 83, 16),
        );

        if lobby_hero(
            ui,
            "CASINO RUST",
            "Entrez dans un lobby pense comme un vrai casino en ligne.",
            "Tables elegantes, ambiance nocturne, jeux classiques et sessions rapides reunis dans une interface unique, sombre et premium.",
            "Decouvrir les tables",
        )
        .clicked()
        {
            ui.scroll_to_cursor(Some(egui::Align::Center));
        }

        ui.add_space(18.0);
        subpanel_frame().show(ui, |ui| {
            section_title(ui, "Jeux disponibles", "Selectionne une table et entre dans le salon.");
        });

        ui.add_space(14.0);
        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                if game_tile(
                    ui,
                    "Poker Texas Hold'em",
                    "Solo contre bots ou online multijoueur.",
                    "TABLE STAR",
                    egui::Color32::from_rgb(22, 121, 81),
                    GameAsset::Poker,
                )
                .clicked()
                {
                    self.ecran = EcranCasino::Poker;
                    self.poker_vue = PokerVue::Choix;
                }
                ui.add_space(78.0);
                if game_tile(
                    ui,
                    "Machine a sous",
                    "Session rapide avec reels et jackpot.",
                    "ARCADE",
                    egui::Color32::from_rgb(200, 134, 36),
                    GameAsset::Slot,
                )
                .clicked()
                {
                    self.ecran = EcranCasino::SlotMachine;
                }
            });

            columns[1].vertical(|ui| {
                if game_tile(
                    ui,
                    "Blackjack",
                    "Table multi-joueurs avec bots et croupier.",
                    "CLASSIQUE",
                    egui::Color32::from_rgb(176, 42, 51),
                    GameAsset::Blackjack,
                )
                .clicked()
                {
                    self.ecran = EcranCasino::Blackjack;
                }
                ui.add_space(78.0);
                if game_tile(
                    ui,
                    "Hi-Lo",
                    "Pari instantane sur la prochaine carte.",
                    "RAPIDE",
                    egui::Color32::from_rgb(74, 108, 201),
                    GameAsset::HiLo,
                )
                .clicked()
                {
                    self.ecran = EcranCasino::HiLo;
                }
            });
        });
    }
}
