use crate::games::blackjack::engine::JeuBlackjack;
use crate::games::crash::engine::JeuCrash;
use crate::games::hilo::AceMode;
use crate::games::mines::engine::JeuMines;
use crate::games::roulette::RouletteResult;
use eframe::egui;
use std::sync::mpsc;
use std::thread;

mod assets;
mod blackjack;
mod crash;
mod draw;
mod hilo;
mod login;
mod mines;
mod poker;
mod poker_online;
mod roulette;
mod slotmachine;
mod theme;

use self::assets::GameAsset;
use self::login::LoginState;
use self::poker::PokerGuiGame;
use self::poker_online::OnlinePokerState;
use self::roulette::RouletteBetUI;
use self::theme::{
    apply_casino_theme, back_button, game_tile, lobby_hero, paint_global_background, panel_frame, premium_button,
    section_title, status_panel, subpanel_frame, BG_DARK, GOLD_SOFT, TEXT_DIM,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EcranCasino {
    Login,
    Menu,
    Poker,
    Blackjack,
    SlotMachine,
    HiLo,
    Roulette,
    Mines,
    Crash,
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

enum WalletSyncRequest {
    Save { db_id: i32, jetons: u32 },
}

struct WalletMovement {
    source: String,
    delta: i32,
    balance_after: u32,
}

pub struct CasinoApp {
    ecran: EcranCasino,
    joueur_pseudo: String,
    joueur_db_id: Option<i32>,
    login: LoginState,
    poker_vue: PokerVue,
    banque_joueur: u32,
    banque_depart_session: u32,
    wallet_history: Vec<WalletMovement>,
    wallet_sync_tx: mpsc::Sender<WalletSyncRequest>,
    wallet_sync_status: String,
    jetons_depart: u32,
    small_blind: u32,
    big_blind: u32,
    poker: Option<PokerGuiGame>,
    poker_wallet_snapshot: Option<u32>,
    poker_online: OnlinePokerState,
    tx_online: Option<mpsc::Sender<crate::network::protocol::ActionJoueur>>,
    rx_online: Option<mpsc::Receiver<crate::network::protocol::MessageServeur>>,
    blackjack: Option<JeuBlackjack>,
    blackjack_wallet_snapshot: Option<u32>,
    bj_nb_joueurs: u8,
    bj_jetons_depart: u32,
    bj_mise_input: u32,
    slot_symbols: [usize; 3],
    slot_result: String,
    slot_mise: u32,
    slot_anim: Option<slotmachine::SlotMachineAnim>,
    hilo: Option<crate::games::hilo::HiLoGame>,
    hilo_wallet_snapshot: Option<u32>,
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
    wallpaper_texture: Option<egui::TextureHandle>,
    roulette_bet: RouletteBetUI,
    roulette_mise: u32,  // Slider value
    roulette_mise_en_jeu: u32,  // Actual deducted bet
    roulette_last_result: Option<RouletteResult>,
    roulette_anim: Option<roulette::RouletteAnim>,
    mines: Option<JeuMines>,
    mines_mise: u32,
    mines_nb_mines: u8,
    mines_client_seed: String,
    mines_nonce: u64,
    crash: JeuCrash,
    crash_mise: u32,
    depot_input: u32,
}

impl Default for CasinoApp {
    fn default() -> Self {
        let wallet_sync_tx = demarrer_wallet_sync_worker();
        Self {
            ecran: EcranCasino::Login,
            joueur_pseudo: String::new(),
            joueur_db_id: None,
            login: LoginState::default(),
            poker_vue: PokerVue::Choix,
            banque_joueur: 0,
            banque_depart_session: 0,
            wallet_history: Vec::new(),
            wallet_sync_tx,
            wallet_sync_status: "Hors ligne".to_string(),
            jetons_depart: 200,
            small_blind: 10,
            big_blind: 20,
            poker: None,
            poker_wallet_snapshot: None,
            poker_online: OnlinePokerState::default(),
            tx_online: None,
            rx_online: None,
            blackjack: None,
            blackjack_wallet_snapshot: None,
            bj_nb_joueurs: 3,
            bj_jetons_depart: 500,
            bj_mise_input: 20,
            slot_symbols: [0, 1, 2],
            slot_result: String::new(),
            slot_mise: 10,
            slot_anim: None,
            hilo: None,
            hilo_wallet_snapshot: None,
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
            wallpaper_texture: None,
            roulette_bet: RouletteBetUI::None,
            roulette_mise: 10,
            roulette_mise_en_jeu: 0,
            roulette_last_result: None,
            roulette_anim: None,
            mines: None,
            mines_mise: 10,
            mines_nb_mines: 3,
            mines_client_seed: "casino-rust".to_string(),
            mines_nonce: 1,
            crash: JeuCrash::default(),
            crash_mise: 10,
            depot_input: 100,
        }
    }
}

impl eframe::App for CasinoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_casino_theme(ctx);
        if self.wallpaper_texture.is_none() {
            if let Ok(image) =
                egui_extras::image::load_image_bytes(include_bytes!("../../../assets/walpaper.png"))
            {
                self.wallpaper_texture = Some(ctx.load_texture(
                    "casino_wallpaper",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        if let Some(texture) = &self.wallpaper_texture {
            paint_global_background(ctx, texture);
        }
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
                    EcranCasino::Login => "Connexion",
                    EcranCasino::Menu => "Menu",
                    EcranCasino::Poker => "Poker Texas Hold'em",
                    EcranCasino::Blackjack => "Blackjack",
                    EcranCasino::SlotMachine => "Machine à sous",
                    EcranCasino::HiLo => "Hi-Lo",
                    EcranCasino::Roulette => "Roulette",
                    EcranCasino::Mines => "Mines",
                    EcranCasino::Crash => "Crash",
                    EcranCasino::Depot => "Depot",
                    })
                    .color(TEXT_DIM),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.ecran != EcranCasino::Login
                        && self.joueur_db_id.is_some()
                        && premium_button(ui, "Depot").clicked()
                    {
                        self.ecran = EcranCasino::Depot;
                    }
                    if self.ecran != EcranCasino::Login {
                        ui.label(
                            egui::RichText::new(format!(
                                "Capital global: {} jetons",
                                self.banque_joueur
                            ))
                            .color(GOLD_SOFT),
                        );
                    }
                    if self.ecran != EcranCasino::Login && !self.joueur_pseudo.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Joueur: {}", self.joueur_pseudo))
                                .color(TEXT_DIM),
                        );
                    }
                });
            });
        });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.ecran {
                        EcranCasino::Login => self.ui_login(ui, ctx),
                        EcranCasino::Menu => self.ui_menu(ui),
                        EcranCasino::Poker => self.ui_poker(ui),
                        EcranCasino::Blackjack => self.ui_blackjack(ui),
                        EcranCasino::SlotMachine => self.ui_slot_machine(ui),
                        EcranCasino::HiLo => self.ui_hilo(ui),
                        EcranCasino::Roulette => self.ui_roulette(ui),
                        EcranCasino::Mines => self.ui_mines(ui),
                        EcranCasino::Crash => self.ui_crash(ui),
                        EcranCasino::Depot => self.ui_depot(ui),
                    });
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(80));
    }
}

impl CasinoApp {
    fn ui_menu(&mut self, ui: &mut egui::Ui) {
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
        ui.columns(3, |columns| {
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
                ui.add_space(78.0);
                if game_tile(
                    ui,
                    "Mines",
                    "Grille 5x5, cash out et mode provably fair.",
                    "RISQUE",
                    egui::Color32::from_rgb(44, 176, 132),
                    GameAsset::Mines,
                )
                .clicked()
                {
                    self.ecran = EcranCasino::Mines;
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
                ui.add_space(78.0);
                if game_tile(
                    ui,
                    "Crash",
                    "Cash out avant l'explosion avec historique des crashs.",
                    "LIVE",
                    egui::Color32::from_rgb(214, 112, 48),
                    GameAsset::Crash,
                )
                .clicked()
                {
                    self.ecran = EcranCasino::Crash;
                }
            });

            columns[2].vertical(|ui| {
                if game_tile(
                    ui,
                    "Roulette europeenne",
                    "Roue, tableau de mises et gains relies au portefeuille global.",
                    "TABLE PRESTIGE",
                    egui::Color32::from_rgb(212, 176, 84),
                    GameAsset::Roulette,
                )
                .clicked()
                {
                    self.ecran = EcranCasino::Roulette;
                }
            });
        });
        ui.add_space(18.0);
    }

    fn ui_depot(&mut self, ui: &mut egui::Ui) {
        panel_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                if back_button(ui, "<- Retour menu").clicked() {
                    self.ecran = EcranCasino::Menu;
                }
                ui.separator();
                ui.heading("Depot");
            });

            ui.add_space(10.0);
            section_title(ui, "Gestion du solde", "Recharge le portefeuille global du joueur.");
            ui.add_space(12.0);

            status_panel(ui, format!("Banque actuelle: {} jetons", self.banque_joueur));
            ui.add_space(10.0);

            subpanel_frame().show(ui, |ui| {
                let gain_session = self.banque_joueur as i64 - self.banque_depart_session as i64;
                ui.label(format!(
                    "Gain session: {}{}",
                    if gain_session >= 0 { "+" } else { "" },
                    gain_session
                ));
                ui.add_space(8.0);
                ui.label("Montant a deposer");
                ui.add(
                    egui::Slider::new(&mut self.depot_input, 10..=10_000)
                        .suffix(" jetons")
                        .text("Depot"),
                );
                ui.add_space(10.0);
                if premium_button(ui, "Crediter le compte").clicked() {
                    self.crediter_banque_joueur_avec_source(self.depot_input, "Depot");
                    self.wallet_sync_status = "Sync demandee".to_string();
                }
            });

            ui.add_space(12.0);
            subpanel_frame().show(ui, |ui| {
                section_title(ui, "Historique portefeuille", "Derniers mouvements credites ou debites par l'interface.");
                ui.add_space(8.0);
                if self.wallet_history.is_empty() {
                    ui.label(egui::RichText::new("Aucun mouvement pour le moment.").color(TEXT_DIM));
                } else {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for entry in self.wallet_history.iter().rev() {
                            let color = if entry.delta >= 0 {
                                egui::Color32::from_rgb(72, 190, 133)
                            } else {
                                egui::Color32::from_rgb(198, 74, 82)
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&entry.source).color(GOLD_SOFT));
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}{} jetons",
                                        if entry.delta >= 0 { "+" } else { "" },
                                        entry.delta
                                    ))
                                    .color(color),
                                );
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Solde: {}",
                                        entry.balance_after
                                    ))
                                    .color(TEXT_DIM),
                                );
                            });
                            ui.add_space(4.0);
                        }
                    });
                }
            });
        });
    }

    fn capital_depart_jeu(&self, fallback: u32) -> u32 {
        if self.joueur_db_id.is_some() {
            self.banque_joueur
        } else {
            fallback
        }
    }

    fn definir_banque_joueur(&mut self, montant: u32) {
        if self.banque_joueur == montant {
            return;
        }
        self.banque_joueur = montant;
        self.sauvegarder_banque_joueur();
    }

    fn crediter_banque_joueur(&mut self, montant: u32) {
        self.crediter_banque_joueur_avec_source(montant, "Credit");
    }

    fn debiter_banque_joueur(&mut self, montant: u32) {
        self.debiter_banque_joueur_avec_source(montant, "Debit");
    }

    fn crediter_banque_joueur_avec_source(&mut self, montant: u32, source: &str) {
        let nouveau = self.banque_joueur.saturating_add(montant);
        if nouveau == self.banque_joueur {
            return;
        }
        self.banque_joueur = nouveau;
        self.enregistrer_mouvement_wallet(source, montant as i32);
        self.sauvegarder_banque_joueur();
    }

    fn debiter_banque_joueur_avec_source(&mut self, montant: u32, source: &str) {
        let reel = montant.min(self.banque_joueur);
        if reel == 0 {
            return;
        }
        self.banque_joueur = self.banque_joueur.saturating_sub(reel);
        self.enregistrer_mouvement_wallet(source, -(reel as i32));
        self.sauvegarder_banque_joueur();
    }

    fn enregistrer_mouvement_wallet(&mut self, source: &str, delta: i32) {
        self.wallet_history.push(WalletMovement {
            source: source.to_string(),
            delta,
            balance_after: self.banque_joueur,
        });
        if self.wallet_history.len() > 32 {
            let surplus = self.wallet_history.len() - 32;
            self.wallet_history.drain(0..surplus);
        }
    }

    fn synchroniser_banque_depuis_jeu(
        &mut self,
        snapshot: &mut Option<u32>,
        montant_courant: u32,
        source: &str,
    ) {
        let precedent = snapshot.unwrap_or(montant_courant);
        if precedent != montant_courant {
            self.enregistrer_mouvement_wallet(source, montant_courant as i32 - precedent as i32);
        }
        *snapshot = Some(montant_courant);
        self.definir_banque_joueur(montant_courant);
    }

    fn sauvegarder_banque_joueur(&self) {
        let Some(db_id) = self.joueur_db_id else {
            return;
        };
        let _ = self.wallet_sync_tx.send(WalletSyncRequest::Save {
            db_id,
            jetons: self.banque_joueur,
        });
    }
}

fn demarrer_wallet_sync_worker() -> mpsc::Sender<WalletSyncRequest> {
    let (tx, rx) = mpsc::channel::<WalletSyncRequest>();

    thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                eprintln!("Wallet sync runtime impossible: {err}");
                return;
            }
        };

        rt.block_on(async move {
            dotenvy::dotenv().ok();
            let url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://poker:poker@localhost:5432/poker".to_string());
            let pool = match sqlx::PgPool::connect(&url).await {
                Ok(pool) => pool,
                Err(err) => {
                    eprintln!("Wallet sync DB inaccessible: {err}");
                    return;
                }
            };

            while let Ok(request) = rx.recv() {
                match request {
                    WalletSyncRequest::Save { db_id, jetons } => {
                        let mut last_db_id = db_id;
                        let mut last_jetons = jetons;

                        while let Ok(WalletSyncRequest::Save { db_id, jetons }) = rx.try_recv() {
                            last_db_id = db_id;
                            last_jetons = jetons;
                        }

                        if let Err(err) =
                            crate::db::joueur_repo::maj_jetons(&pool, last_db_id, last_jetons as i32)
                                .await
                        {
                            eprintln!("Erreur sync wallet joueur {last_db_id}: {err}");
                        }
                    }
                }
            }
        });
    });

    tx
}
