use crate::games::mines::engine::{CaseMine, EtatMines, JeuMines};
use eframe::egui;
use super::assets::{paint_contained_art, GameAsset};
use super::theme::{
    back_button, panel_frame, premium_button, section_title, status_panel, subpanel_frame,
    GOLD_SOFT, TEXT_DIM,
};

impl super::CasinoApp {
    pub(super) fn ui_mines(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if back_button(ui, "<- Retour menu").clicked() {
                self.ecran = super::EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Mines");
        });

        status_panel(
            ui,
            format!(
                "Capital global: {} jetons | Mise: {} | Mines: {}",
                self.banque_joueur, self.mines_mise, self.mines_nb_mines
            ),
        );
        ui.add_space(8.0);

        panel_frame().show(ui, |ui| {
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    section_title(ui, "Configuration", "Lance une grille 5x5 et encaisse avant de tomber sur une mine.");
                    ui.add_space(10.0);
                    subpanel_frame().show(ui, |ui| {
                        let max_mise = self.banque_joueur.max(1);
                        if self.mines_mise > max_mise {
                            self.mines_mise = max_mise;
                        }
                        ui.add(egui::Slider::new(&mut self.mines_mise, 1..=max_mise).text("Mise"));
                        ui.add(egui::Slider::new(&mut self.mines_nb_mines, 1..=24).text("Nombre de mines"));
                        ui.horizontal(|ui| {
                            ui.label("Client seed:");
                            ui.text_edit_singleline(&mut self.mines_client_seed);
                        });
                        ui.add_space(8.0);
                        if premium_button(ui, "Nouvelle grille").clicked() {
                            let mise = self.mines_mise as f64;
                            if mise > self.banque_joueur as f64 {
                                return;
                            }
                            match JeuMines::nouveau(
                                self.mines_nb_mines,
                                mise,
                                self.mines_client_seed.clone(),
                                format!("server-seed-{}", self.mines_nonce),
                                self.mines_nonce,
                            ) {
                                Ok(game) => {
                                    self.debiter_banque_joueur_avec_source(self.mines_mise, "Mines - Mise");
                                    self.mines = Some(game);
                                    self.mines_nonce = self.mines_nonce.saturating_add(1);
                                }
                                Err(_) => {}
                            }
                        }
                    });
                });

                columns[1].vertical(|ui| {
                    subpanel_frame().show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Gemmes et mines")
                                .size(20.0)
                                .strong()
                                .color(GOLD_SOFT),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Le diamant represente la progression sure. L'important est d'encaisser avant le mauvais clic.",
                            )
                            .color(TEXT_DIM),
                        );
                        ui.add_space(12.0);
                        let (art_rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 160.0),
                            egui::Sense::hover(),
                        );
                        paint_contained_art(ui, art_rect.shrink(4.0), GameAsset::Diamond, 18);
                    });
                });
            });
        });

        let mut credit = None;
        let mut reset = false;

        if let Some(game) = &mut self.mines {
            ui.add_space(10.0);
            status_panel(ui, &game.message);
            ui.add_space(10.0);

            panel_frame().show(ui, |ui| {
                section_title(ui, "Grille", "Revele des cases et encaisse ton multiplicateur courant.");
                ui.add_space(12.0);
                for row in 0..5 {
                    ui.horizontal(|ui| {
                        for col in 0..5 {
                            let (label, fill) = match game.grille[row][col] {
                                CaseMine::Cachee => ("?", egui::Color32::from_rgb(28, 37, 49)),
                                CaseMine::Revelee => ("DIAM", egui::Color32::from_rgb(54, 148, 105)),
                                CaseMine::MineRevelee => ("X", egui::Color32::from_rgb(176, 42, 51)),
                                CaseMine::MineMontree => ("*", egui::Color32::from_rgb(113, 57, 63)),
                            };
                            let response = ui.add_sized(
                                [56.0, 56.0],
                                egui::Button::new(label).fill(fill),
                            );
                            if response.clicked() && game.etat == EtatMines::Actif {
                                if let Ok(mult) = game.reveler(row, col) {
                                    if matches!(game.etat, EtatMines::Perdu) {
                                        reset = false;
                                    } else if matches!(game.etat, EtatMines::Gagne(_)) {
                                        credit = Some(game.paiement().round() as u32);
                                    } else {
                                        game.message = format!(
                                            "Gemme trouvee. Multiplicateur: {:.4}x",
                                            mult
                                        );
                                    }
                                }
                            }
                        }
                    });
                    ui.add_space(6.0);
                }
            });

            ui.add_space(10.0);
            subpanel_frame().show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Multiplicateur: {:.4}x | Paiement potentiel: {:.2}",
                        game.multiplicateur,
                        game.mise * game.multiplicateur
                    ))
                    .color(GOLD_SOFT),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(game.etat == EtatMines::Actif && game.cases_revelees > 0, egui::Button::new("Encaisser"))
                        .clicked()
                    {
                        if let Ok(paiement) = game.encaisser() {
                            credit = Some(paiement.round() as u32);
                        }
                    }
                    if ui.button("Autoplay 3").clicked() && game.etat == EtatMines::Actif {
                        game.autoplay(3);
                        if matches!(game.etat, EtatMines::Gagne(_)) {
                            credit = Some(game.paiement().round() as u32);
                        }
                    }
                    if ui.button("Fermer la table").clicked() {
                        reset = true;
                    }
                });
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Provably fair: {}",
                        if game.verifier_equite() { "valide" } else { "invalide" }
                    ))
                    .color(TEXT_DIM),
                );
            });
        }

        if let Some(amount) = credit {
            self.crediter_banque_joueur_avec_source(amount, "Mines - Gain");
        }

        if reset {
            self.mines = None;
        }
    }
}
