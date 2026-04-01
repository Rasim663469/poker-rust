use crate::games::blackjack::engine::{EtatBlackjack, JeuBlackjack};
use eframe::egui;
use super::draw::{dessiner_carte, dessiner_jetons, dessiner_zone_label};
use super::theme::{back_button, panel_frame, premium_button, section_title, status_panel, subpanel_frame, TABLE_GREEN, TEXT_DIM};

impl super::CasinoApp {
    pub(super) fn ui_blackjack(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if back_button(ui, "<- Retour menu").clicked() {
                self.ecran = super::EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Blackjack");
        });

        if self.blackjack.is_none() {
            panel_frame().show(ui, |ui| {
                section_title(ui, "Table Blackjack", "Configure une table multi-joueurs avec bots.");
                ui.add_space(10.0);
                subpanel_frame().show(ui, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.bj_nb_joueurs)
                            .range(2..=6)
                            .prefix("Joueurs: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.bj_jetons_depart)
                            .range(50..=10_000)
                            .prefix("Jetons: "),
                    );
                });
                ui.add_space(10.0);
                if premium_button(ui, "Creer table Blackjack").clicked()
                {
                    let jetons_depart = self.capital_depart_jeu(self.bj_jetons_depart);
                    self.blackjack_wallet_snapshot = Some(jetons_depart);
                    self.blackjack = Some(JeuBlackjack::nouveau(
                        self.bj_nb_joueurs as usize,
                        jetons_depart,
                    ));
                }
            });
            return;
        }

        let Some(jeu) = &mut self.blackjack else {
            return;
        };
        ui.separator();
        status_panel(
            ui,
            format!(
                "Jetons (toi): {} | Mise de reference: {}",
                jeu.jetons_humain(),
                jeu.mise_reference
            ),
        );
        ui.add_space(6.0);
        subpanel_frame().show(ui, |ui| {
            ui.label(&jeu.message);
        });
        ui.add_space(8.0);

        let table_height = 500.0;
        let table_width = (ui.available_width() - 4.0).max(760.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(table_width, table_height), egui::Sense::hover());
        dessiner_table_blackjack(ui, rect, jeu);

        ui.add_space(16.0);
        panel_frame().show(ui, |ui| {
            section_title(ui, "Actions", "Commandes de manche.");
            ui.add_space(10.0);
            subpanel_frame().show(ui, |ui| {
                ui.set_min_height(88.0);
                if jeu.etat == EtatBlackjack::EnAttenteMise || jeu.etat == EtatBlackjack::Termine {
                    let max_mise = jeu.jetons_humain().max(1);
                    if self.bj_mise_input == 0 || self.bj_mise_input > max_mise {
                        self.bj_mise_input = 1.min(max_mise);
                    }
                    ui.label("Nouvelle manche");
                    ui.add_space(6.0);
                    ui.add(
                        egui::Slider::new(&mut self.bj_mise_input, 1..=max_mise).text("Mise"),
                    );
                    ui.add_space(10.0);
                    if premium_button(ui, "Distribuer").clicked() {
                        let _ = jeu.commencer_manche(self.bj_mise_input);
                    }
                } else if jeu.etat == EtatBlackjack::TourJoueur && jeu.est_tour_humain() {
                    ui.label("Ton tour");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if premium_button(ui, "Hit").clicked() {
                            jeu.joueur_hit();
                        }
                        if premium_button(ui, "Stand").clicked() {
                            jeu.joueur_stand();
                        }
                    });
                } else {
                    ui.label("Tour en cours");
                    ui.add_space(8.0);
                    ui.colored_label(TEXT_DIM, "Les bots et le croupier jouent.");
                }
            });
        });

        let jetons_humain = jeu.jetons_humain();
        let _ = jeu;
        let mut snapshot = self.blackjack_wallet_snapshot;
        self.synchroniser_banque_depuis_jeu(
            &mut snapshot,
            jetons_humain,
            "Blackjack - Resultat",
        );
        self.blackjack_wallet_snapshot = snapshot;
    }
}

fn dessiner_table_blackjack(ui: &mut egui::Ui, rect: egui::Rect, jeu: &JeuBlackjack) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 18.0, egui::Color32::from_rgb(12, 28, 24));
    let table = rect.shrink2(egui::vec2(18.0, 12.0));
    painter.rect_filled(table, 120.0, TABLE_GREEN);
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
    let zone_y = table.bottom() - 74.0;
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
        let zone =
            egui::Rect::from_center_size(egui::pos2(x_center, zone_y), egui::vec2(zone_w, 44.0));
        let titre = if *idx == 0 {
            format!("Toi | Score: {}", jeu.score_joueur(*idx))
        } else {
            format!("{} | Score: {}", j.nom, jeu.score_joueur(*idx))
        };
        dessiner_zone_label(&painter, zone, &titre);

        let cards_y = zone.bottom() + 12.0;
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
