use crate::games::hilo::{AceMode, HiLoConfig, HiLoGame, HiLoGuess, HiLoState};
use eframe::egui;
use super::draw::dessiner_carte;
use super::theme::{back_button, panel_frame, premium_button, section_title, status_panel, subpanel_frame, TABLE_GREEN};

impl super::CasinoApp {
    pub(super) fn ui_hilo(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if back_button(ui, "<- Retour menu").clicked() {
                self.ecran = super::EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Hi-Lo");
        });

        if self.hilo.is_none() {
            panel_frame().show(ui, |ui| {
                section_title(ui, "Hi-Lo", "Version rapide higher / lower avec options de payout.");
                ui.add_space(8.0);

                subpanel_frame().show(ui, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.hilo_jetons_depart)
                            .range(50..=10_000)
                            .prefix("Jetons depart: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.hilo_min_bet)
                            .range(1..=10_000)
                            .prefix("Mise min: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.hilo_max_bet)
                            .range(self.hilo_min_bet..=50_000)
                            .prefix("Mise max: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.hilo_payout_win)
                            .range(1..=10)
                            .prefix("Payout win x"),
                    );
                    ui.checkbox(&mut self.hilo_allow_equal, "Autoriser Equal");
                    ui.add_enabled(
                        self.hilo_allow_equal,
                        egui::DragValue::new(&mut self.hilo_payout_equal)
                            .range(2..=20)
                            .prefix("Payout equal x"),
                    );
                    ui.horizontal(|ui| {
                        ui.label("As:");
                        ui.radio_value(&mut self.hilo_ace_mode, AceMode::High, "Haut");
                        ui.radio_value(&mut self.hilo_ace_mode, AceMode::Low, "Bas");
                    });
                });

                ui.add_space(8.0);
                if premium_button(ui, "Creer une table").clicked()
                {
                    let mut game = HiLoGame::new_with_config(
                        self.hilo_jetons_depart,
                        HiLoConfig {
                            allow_equal: self.hilo_allow_equal,
                            ace_mode: self.hilo_ace_mode,
                            payout_win: self.hilo_payout_win,
                            payout_equal: self.hilo_payout_equal,
                            min_bet: self.hilo_min_bet,
                            max_bet: self.hilo_max_bet,
                        },
                    );
                    if game.config.min_bet > game.config.max_bet {
                        game.config.max_bet = game.config.min_bet;
                    }
                    self.hilo = Some(game);
                }
            });
            return;
        }

        let mut reset_table = false;
        let game = self.hilo.as_mut().expect("hilo must be Some here");

        ui.separator();
        status_panel(ui, format!("Jetons: {} | Streak: {}", game.jetons, game.streak));
        ui.add_space(6.0);
        subpanel_frame().show(ui, |ui| {
            ui.label(&game.message);
        });

        let table_height = 280.0;
        let table_width = (ui.available_width() - 12.0).max(520.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(table_width, table_height), egui::Sense::hover());
        dessiner_table_hilo(ui, rect, game, self.hilo_reveal_at);

        ui.add_space(8.0);
        ui.separator();

        match game.etat {
            HiLoState::EnAttenteMise => {
                let max = game.jetons.max(1).min(game.config.max_bet);
                if self.hilo_mise_input == 0 || self.hilo_mise_input > max {
                    self.hilo_mise_input = game.config.min_bet.min(max);
                }
                ui.label(format!(
                    "Mise ({}..={}):",
                    game.config.min_bet, game.config.max_bet
                ));
                ui.add(egui::Slider::new(&mut self.hilo_mise_input, 1..=max));
                ui.horizontal(|ui| {
                    if ui.button("Valider mise").clicked() {
                        let _ = game.start_round(self.hilo_mise_input);
                    }
                    if ui.button("Rebet").clicked() {
                        let _ = game.rebet();
                    }
                });
            }
            HiLoState::EnAttenteChoix => {
                ui.label("Carte suivante: plus haute ou plus basse ?");
                ui.horizontal(|ui| {
                    if ui.button("Higher").clicked() {
                        self.hilo_last_outcome = game.guess(HiLoGuess::Higher).ok();
                        self.hilo_reveal_at = Some(std::time::Instant::now());
                    }
                    if ui.button("Lower").clicked() {
                        self.hilo_last_outcome = game.guess(HiLoGuess::Lower).ok();
                        self.hilo_reveal_at = Some(std::time::Instant::now());
                    }
                    if game.config.allow_equal && ui.button("Equal").clicked() {
                        self.hilo_last_outcome = game.guess(HiLoGuess::Equal).ok();
                        self.hilo_reveal_at = Some(std::time::Instant::now());
                    }
                });
            }
            HiLoState::Resultat => {
                if ui.button("Nouvelle manche").clicked() {
                    game.reset_round();
                }
            }
        }

        ui.add_space(6.0);
        if ui.button("Reinitialiser table").clicked() {
            reset_table = true;
        }

        ui.add_space(10.0);
        ui.label("Historique:");
        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
            for h in game.history.iter().rev() {
                let tag = if h.win { "WIN" } else if h.tie { "TIE" } else { "LOSE" };
                ui.label(format!(
                    "{}: {} -> {} | {:?} | +{}",
                    tag,
                    h.current,
                    h.next,
                    h.guess,
                    h.payout
                ));
            }
        });

        if reset_table {
            if let Some(game) = &self.hilo {
                self.banque_joueur += game.jetons;
            }
            self.hilo = None;
            self.hilo_last_outcome = None;
            self.hilo_reveal_at = None;
        }
    }
}

fn dessiner_table_hilo(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    game: &HiLoGame,
    reveal_at: Option<std::time::Instant>,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 18.0, egui::Color32::from_rgb(12, 28, 24));

    let table = rect.shrink2(egui::vec2(16.0, 12.0));
    painter.rect_filled(table, 80.0, TABLE_GREEN);
    painter.rect_stroke(
        table,
        80.0,
        egui::Stroke::new(3.0, egui::Color32::from_rgb(132, 85, 50)),
        egui::StrokeKind::Outside,
    );

    let c = table.center();
    let current_rect = egui::Rect::from_min_size(
        egui::pos2(c.x - 110.0, c.y - 40.0),
        egui::vec2(80.0, 120.0),
    );
    let next_rect = egui::Rect::from_min_size(
        egui::pos2(c.x + 30.0, c.y - 40.0),
        egui::vec2(80.0, 120.0),
    );

    let current = game.current.as_ref();
    dessiner_carte(ui, &painter, current_rect, current, current.is_some());

    let show_next = match reveal_at {
        Some(t) => t.elapsed().as_millis() > 450,
        None => game.etat == HiLoState::Resultat,
    };
    let next = if show_next { game.next.as_ref() } else { None };
    dessiner_carte(ui, &painter, next_rect, next, show_next);

    if let Some(outcome) = &game.last_outcome {
        if show_next && outcome.win {
            painter.rect_stroke(
                next_rect.expand(3.0),
                10.0,
                egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 215, 0)),
                egui::StrokeKind::Outside,
            );
        }
    }

    painter.text(
        egui::pos2(c.x - 70.0, table.bottom() - 20.0),
        egui::Align2::CENTER_CENTER,
        "Carte courante",
        egui::FontId::proportional(14.0),
        egui::Color32::from_rgb(220, 232, 227),
    );
    painter.text(
        egui::pos2(c.x + 70.0, table.bottom() - 20.0),
        egui::Align2::CENTER_CENTER,
        if show_next { "Carte suivante" } else { "?" },
        egui::FontId::proportional(14.0),
        egui::Color32::from_rgb(220, 232, 227),
    );
}
