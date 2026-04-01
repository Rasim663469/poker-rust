use crate::games::crash::engine::{EtatCrash, JeuCrash};
use eframe::egui;

use super::assets::{paint_contained_art, GameAsset};
use super::theme::{
    back_button, panel_frame, premium_button, section_title, status_panel, subpanel_frame,
    GOLD_SOFT, TEXT_DIM,
};

impl super::CasinoApp {
    pub(super) fn ui_crash(&mut self, ui: &mut egui::Ui) {
        if self.crash.est_en_vol() {
            self.crash.avancer(0.08);
        }

        ui.horizontal(|ui| {
            if back_button(ui, "<- Retour menu").clicked() {
                self.ecran = super::EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Crash");
        });

        ui.add_space(8.0);
        status_panel(
            ui,
            format!(
                "Capital global: {} jetons | Mise: {} | Multiplicateur: {:.2}x",
                self.banque_joueur,
                self.crash_mise,
                self.crash.multiplicateur
            ),
        );
        ui.add_space(10.0);

        panel_frame().show(ui, |ui| {
            section_title(
                ui,
                "Vol en direct",
                "Lance une manche, laisse monter le multiplicateur et encaisse avant l'explosion.",
            );
            ui.add_space(12.0);

            subpanel_frame().show(ui, |ui| {
                let (art_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 140.0),
                    egui::Sense::hover(),
                );
                paint_contained_art(ui, art_rect.shrink(4.0), GameAsset::Plane, 18);
            });
            ui.add_space(12.0);

            let (hero_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 260.0),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(hero_rect);
            painter.rect_filled(hero_rect, 24.0, egui::Color32::from_rgb(14, 31, 45));
            painter.rect_stroke(
                hero_rect,
                24.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(214, 112, 48)),
                egui::StrokeKind::Outside,
            );
            painter.circle_filled(
                egui::pos2(hero_rect.right() - 130.0, hero_rect.top() + 86.0),
                92.0,
                egui::Color32::from_rgba_premultiplied(214, 112, 48, 22),
            );

            let curve_left = hero_rect.left() + 36.0;
            let curve_bottom = hero_rect.bottom() - 48.0;
            let curve_width = hero_rect.width() - 120.0;
            let curve_height = 132.0;
            let points = (0..=48)
                .map(|i| {
                    let t = i as f32 / 48.0;
                    let x = curve_left + curve_width * t;
                    let growth = ((self.crash.multiplicateur as f32 - 1.0).max(0.0) / 8.0).min(1.0);
                    let eased = t * t * (0.65 + growth * 0.75);
                    let y = curve_bottom - curve_height * eased;
                    egui::pos2(x, y)
                })
                .collect::<Vec<_>>();
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(4.0, egui::Color32::from_rgb(240, 220, 158)),
            ));

            painter.text(
                egui::pos2(hero_rect.left() + 28.0, hero_rect.top() + 22.0),
                egui::Align2::LEFT_TOP,
                format!("{:.2}x", self.crash.multiplicateur),
                egui::FontId::proportional(42.0),
                GOLD_SOFT,
            );
            painter.text(
                egui::pos2(hero_rect.left() + 30.0, hero_rect.top() + 74.0),
                egui::Align2::LEFT_TOP,
                match self.crash.etat {
                    EtatCrash::EnAttente => "En attente d'une nouvelle manche",
                    EtatCrash::EnVol => "Le vol est en cours",
                    EtatCrash::Encaisse { .. } => "Cash out valide, attente du crash final",
                    EtatCrash::Explose { .. } => "Le vol a explose",
                },
                egui::FontId::proportional(18.0),
                TEXT_DIM,
            );

            ui.add_space(14.0);
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    subpanel_frame().show(ui, |ui| {
                        section_title(ui, "Manche", "Le portefeuille global est debite au lancement.");
                        ui.add_space(8.0);
                        let max_mise = self.banque_joueur.max(1);
                        if !self.crash.manche_en_cours() && self.crash_mise > max_mise {
                            self.crash_mise = max_mise;
                        }
                        ui.add(
                            egui::Slider::new(&mut self.crash_mise, 1..=max_mise)
                                .text("Mise")
                                .suffix(" jetons"),
                        );
                        ui.add_space(10.0);

                        match self.crash.etat {
                            EtatCrash::EnAttente | EtatCrash::Explose { .. } => {
                                if premium_button(ui, "Lancer le vol").clicked()
                                    && self.banque_joueur >= self.crash_mise
                                {
                                    self.debiter_banque_joueur_avec_source(
                                        self.crash_mise,
                                        "Crash - Mise",
                                    );
                                    match self.crash.lancer_tour(self.crash_mise as f64) {
                                        Ok(()) => {}
                                        Err(err) => {
                                            self.crediter_banque_joueur_avec_source(
                                                self.crash_mise,
                                                "Crash - Annulation",
                                            );
                                            self.crash.message = err;
                                        }
                                    }
                                }
                            }
                            EtatCrash::EnVol => {
                                if premium_button(ui, "Cash out").clicked() {
                                    if let Ok(paiement) = self.crash.encaisser() {
                                        self.crediter_banque_joueur_avec_source(
                                            paiement.round() as u32,
                                            "Crash - Cash out",
                                        );
                                    }
                                }
                            }
                            EtatCrash::Encaisse { paiement } => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Paiement verrouille: {:.2}",
                                        paiement
                                    ))
                                    .color(GOLD_SOFT),
                                );
                            }
                        }
                    });
                });

                columns[1].vertical(|ui| {
                    subpanel_frame().show(ui, |ui| {
                        section_title(ui, "Historique", "Derniers points de crash observes.");
                        ui.add_space(8.0);
                        if self.crash.historique.is_empty() {
                            ui.label(egui::RichText::new("Aucune manche jouee.").color(TEXT_DIM));
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                for value in self.crash.historique.iter().rev() {
                                    let color = if *value < 2.0 {
                                        egui::Color32::from_rgb(176, 42, 51)
                                    } else if *value < 10.0 {
                                        egui::Color32::from_rgb(214, 112, 48)
                                    } else {
                                        egui::Color32::from_rgb(44, 176, 132)
                                    };
                                    let text = egui::RichText::new(format!("{value:.2}x"))
                                        .color(color)
                                        .strong();
                                    ui.add(
                                        egui::Label::new(text).sense(egui::Sense::hover()),
                                    );
                                    ui.add_space(6.0);
                                }
                            });
                        }

                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(&self.crash.message)
                                .color(TEXT_DIM)
                                .size(15.0),
                        );
                        if let Some(point) = self.crash.point_crash_revele() {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(format!("Crash final: {point:.2}x"))
                                    .color(GOLD_SOFT),
                            );
                        }
                    });
                });
            });
        });

        if self.crash.est_en_vol() || matches!(self.crash.etat, EtatCrash::Encaisse { .. }) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(80));
        }
    }
}
