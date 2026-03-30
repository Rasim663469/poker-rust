use crate::games::crash::engine::{EtatCrash, JeuCrash};
use eframe::egui;

impl super::CasinoApp {
    pub(super) fn ui_crash(&mut self, ui: &mut egui::Ui) {
        let anim_t = ui.ctx().input(|i| i.time) as f32;

        ui.horizontal(|ui| {
            let btn_retour = crash_button_style(
                egui::RichText::new("<- Retour menu")
                    .strong()
                    .color(egui::Color32::from_rgb(225, 232, 238)),
                egui::Color32::from_rgb(148, 163, 184),
                egui::vec2(170.0, 36.0),
            );
            if ui.add(btn_retour).clicked() {
                self.ecran = super::EcranCasino::Menu;
            }
            ui.separator();
            ui.heading("Crash");
            ui.separator();
            ui.label("Sors avant l'explosion");
        });

        ui.separator();
        let payout_affiche = match self.crash.etat {
            EtatCrash::Encaisse { paiement } => paiement,
            EtatCrash::Explose { .. } => 0.0,
            EtatCrash::EnVol => self.crash.mise * self.crash.multiplicateur,
            EtatCrash::EnAttente => 0.0,
        };
        let multiplicateur_accent = match self.crash.etat {
            EtatCrash::Explose { .. } => egui::Color32::from_rgb(231, 76, 60),
            EtatCrash::Encaisse { .. } => egui::Color32::from_rgb(46, 204, 113),
            _ => egui::Color32::from_rgb(46, 204, 113),
        };
        let payout_accent = match self.crash.etat {
            EtatCrash::Explose { .. } => egui::Color32::from_rgb(231, 76, 60),
            EtatCrash::Encaisse { .. } => egui::Color32::from_rgb(46, 204, 113),
            _ => egui::Color32::from_rgb(245, 176, 65),
        };

        let controls_fixed_h = 192.0;
        let top_area_h = (ui.available_height() - controls_fixed_h - 8.0).max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), top_area_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let zone_w = ui.available_width();
                let kpi_h = 74.0;
                let mult_h = 58.0;
                let info_h = 56.0;
                let hist_h = 74.0;

                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, kpi_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.columns(4, |cols| {
                            ui_crash_indicateur(
                                &mut cols[0],
                                "BALANCE",
                                format!("{:>8.2}", self.crash_solde),
                                egui::Color32::from_rgb(247, 211, 88),
                            );
                            ui_crash_indicateur(
                                &mut cols[1],
                                "MISE",
                                format!("{:>8.2}", self.crash_mise_input),
                                egui::Color32::from_rgb(93, 173, 226),
                            );
                            ui_crash_indicateur(
                                &mut cols[2],
                                "MULTIPLICATEUR",
                                format!("{:>6.2}x", self.crash.multiplicateur),
                                multiplicateur_accent,
                            );
                            ui_crash_indicateur(
                                &mut cols[3],
                                "PAYOUT",
                                format!("{:>8.2}", payout_affiche),
                                payout_accent,
                            );
                        });
                    },
                );

                ui.add_space(4.0);
                let pulsation = if self.crash.est_en_vol() {
                    0.35 + 0.65 * (anim_t * 6.0).sin().abs()
                } else {
                    0.0
                };
                let mult_color = match self.crash.etat {
                    EtatCrash::Explose { .. } => egui::Color32::from_rgb(231, 76, 60),
                    EtatCrash::Encaisse { .. } => egui::Color32::from_rgb(46, 204, 113),
                    _ if self.crash.multiplicateur >= 2.0 => egui::Color32::from_rgb(
                        80,
                        (200.0 + 55.0 * pulsation) as u8,
                        (120.0 + 90.0 * pulsation) as u8,
                    ),
                    _ => egui::Color32::from_rgb(
                        (200.0 + 55.0 * pulsation) as u8,
                        (90.0 + 40.0 * pulsation) as u8,
                        90,
                    ),
                };
                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, mult_h),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:>6.2}x", self.crash.multiplicateur))
                                .size(40.0)
                                .monospace()
                                .strong()
                                .color(mult_color),
                        );
                    },
                );

                ui.add_space(4.0);
                let graph_h = (ui.available_height() - info_h - hist_h - 8.0).max(96.0);
                let graph_w = zone_w.max(500.0);
                let (graph_rect, _) =
                    ui.allocate_exact_size(egui::vec2(graph_w, graph_h), egui::Sense::hover());
                dessiner_graphique_crash(ui, graph_rect, &self.crash);

                ui.add_space(6.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, info_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if !self.crash.message.is_empty() {
                            ui.add_sized(
                                [(zone_w - 4.0).max(0.0), 24.0],
                                egui::Label::new(
                                    egui::RichText::new(&self.crash.message)
                                        .size(17.0)
                                        .strong()
                                        .color(match self.crash.etat {
                                            EtatCrash::Explose { .. } => {
                                                egui::Color32::from_rgb(231, 76, 60)
                                            }
                                            EtatCrash::Encaisse { .. } => {
                                                egui::Color32::from_rgb(46, 204, 113)
                                            }
                                            _ => egui::Color32::from_rgb(220, 232, 227),
                                        }),
                                )
                                .truncate(),
                            );
                        } else {
                            ui.add_space(22.0);
                        }

                        ui.horizontal(|ui| {
                            if let Some(point) = self.crash.point_crash_revele() {
                                ui.monospace(format!("Point d'explosion: {:.2}x", point));
                            }
                            if !self.crash_ui_erreur.is_empty() {
                                ui.colored_label(
                                    egui::Color32::from_rgb(231, 76, 60),
                                    &self.crash_ui_erreur,
                                );
                            }
                        });
                    },
                );

                ui.add_space(2.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(zone_w, hist_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Historique (ligne)").strong());
                                ui.separator();
                                ui.colored_label(egui::Color32::from_rgb(231, 76, 60), "< 2.00x");
                                ui.separator();
                                ui.colored_label(egui::Color32::from_rgb(46, 204, 113), ">= 2.00x");
                            });

                            egui::ScrollArea::horizontal()
                                .max_height(38.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if self.crash.historique.is_empty() {
                                            ui.label("Aucune manche terminee");
                                            return;
                                        }

                                        for x in self.crash.historique.iter().rev().take(12) {
                                            let valeur = format_multiplicateur_historique(*x);
                                            let (stroke, texte) = if *x >= 2.0 {
                                                (
                                                    egui::Color32::from_rgb(102, 255, 163),
                                                    egui::RichText::new(valeur).strong().color(
                                                        egui::Color32::from_rgb(225, 255, 238),
                                                    ),
                                                )
                                            } else {
                                                (
                                                    egui::Color32::from_rgb(255, 142, 142),
                                                    egui::RichText::new(valeur).strong().color(
                                                        egui::Color32::from_rgb(255, 232, 232),
                                                    ),
                                                )
                                            };

                                            let badge = crash_button_style(
                                                texte,
                                                stroke,
                                                egui::vec2(84.0, 30.0),
                                            );
                                            let _ = ui.add(badge);
                                        }
                                    });
                                });
                        });
                    },
                );
            },
        );

        ui.add_space(6.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), controls_fixed_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Gestion de la partie")
                            .strong()
                            .size(16.0),
                    );
                    ui.add_space(6.0);

                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            ui.label(egui::RichText::new("Mise").strong());
                            ui.add_sized(
                                [ui.available_width().max(170.0), 32.0],
                                egui::DragValue::new(&mut self.crash_mise_input)
                                    .range(0.01..=1_000_000.0)
                                    .speed(1.0)
                                    .max_decimals(2),
                            );
                            ui.horizontal(|ui| {
                                if ui.small_button("1/2").clicked() {
                                    self.crash_mise_input = (self.crash_mise_input / 2.0).max(0.01);
                                }
                                if ui.small_button("x2").clicked() {
                                    self.crash_mise_input =
                                        (self.crash_mise_input * 2.0).min(1_000_000.0);
                                }
                                if ui.small_button("MAX").clicked() {
                                    self.crash_mise_input =
                                        self.crash_solde.clamp(0.01, 1_000_000.0);
                                }
                            });
                            ui.add_space(8.0);
                            ui.checkbox(&mut self.crash_auto_actif, "Auto cash out");
                            ui.add_enabled_ui(self.crash_auto_actif, |ui| {
                                ui.add_sized(
                                    [ui.available_width().max(170.0), 32.0],
                                    egui::DragValue::new(&mut self.crash_auto_cashout)
                                        .range(1.01..=100.0)
                                        .speed(0.05)
                                        .max_decimals(2)
                                        .suffix("x"),
                                );
                            });
                            let auto_cible = self.crash_auto_cashout.max(1.01);
                            let auto_texte = if self.crash_auto_actif {
                                format!("Auto cash out a {:.2}x", auto_cible)
                            } else {
                                "Auto cash out desactive".to_string()
                            };
                            ui.label(
                                egui::RichText::new(auto_texte)
                                    .color(egui::Color32::from_rgb(148, 163, 184)),
                            );
                        });

                        cols[1].vertical(|ui| {
                            let w = ui.available_width().max(210.0);

                            let btn_lancer = crash_button_style(
                                egui::RichText::new("Lancer le vol")
                                    .strong()
                                    .size(18.0)
                                    .color(egui::Color32::WHITE),
                                egui::Color32::from_rgb(93, 173, 226),
                                egui::vec2(w, 42.0),
                            )
                            .fill(egui::Color32::from_rgb(19, 31, 45));
                            if ui
                                .add_enabled(!self.crash.manche_en_cours(), btn_lancer)
                                .clicked()
                            {
                                self.crash_ui_erreur.clear();
                                if self.crash_mise_input > self.crash_solde {
                                    self.crash_ui_erreur = format!(
                                        "Solde insuffisant: balance {:.2}, mise {:.2}.",
                                        self.crash_solde, self.crash_mise_input
                                    );
                                } else if let Err(e) = self.crash.lancer_tour(self.crash_mise_input)
                                {
                                    self.crash_ui_erreur = e;
                                } else {
                                    self.crash_solde -= self.crash_mise_input;
                                }
                            }

                            ui.add_space(8.0);
                            let cash_pulse = if self.crash.est_en_vol() {
                                0.45 + 0.55 * (anim_t * 8.0).sin().abs()
                            } else {
                                0.0
                            };
                            let btn_cashout = crash_button_style(
                                egui::RichText::new("CASH OUT")
                                    .strong()
                                    .size(20.0)
                                    .color(egui::Color32::WHITE),
                                egui::Color32::from_rgb(
                                    (120.0 + 45.0 * cash_pulse) as u8,
                                    (215.0 + 35.0 * cash_pulse) as u8,
                                    (155.0 + 35.0 * cash_pulse) as u8,
                                ),
                                egui::vec2(w, 50.0),
                            )
                            .fill(egui::Color32::from_rgb(
                                (20.0 + 12.0 * cash_pulse) as u8,
                                (44.0 + 18.0 * cash_pulse) as u8,
                                (37.0 + 12.0 * cash_pulse) as u8,
                            ))
                            .stroke(egui::Stroke::new(
                                2.0,
                                egui::Color32::from_rgb(183, 255, 214),
                            ))
                            .min_size(egui::vec2(w, 50.0));
                            if ui
                                .add_enabled(self.crash.est_en_vol(), btn_cashout)
                                .clicked()
                            {
                                match self.crash.encaisser() {
                                    Ok(p) => {
                                        self.crash_solde += p;
                                        self.crash_ui_erreur.clear();
                                    }
                                    Err(e) => {
                                        self.crash_ui_erreur = e;
                                    }
                                }
                            }
                        });
                    });
                });
            },
        );
    }
}

fn crash_button_style(
    texte: egui::RichText,
    accent: egui::Color32,
    taille: egui::Vec2,
) -> egui::Button<'static> {
    egui::Button::new(texte)
        .fill(egui::Color32::from_rgb(19, 31, 45))
        .stroke(egui::Stroke::new(1.2, accent))
        .min_size(taille)
}

fn ui_crash_indicateur(ui: &mut egui::Ui, titre: &str, valeur: String, accent: egui::Color32) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(19, 31, 45))
        .stroke(egui::Stroke::new(1.0, accent))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(150.0, 58.0));
            ui.set_max_height(58.0);
            ui.set_width(150.0);
            ui.add_sized(
                [138.0, 13.0],
                egui::Label::new(
                    egui::RichText::new(titre)
                        .size(11.0)
                        .strong()
                        .color(egui::Color32::from_rgb(160, 174, 192)),
                )
                .truncate()
                .selectable(false),
            );
            ui.add_space(1.0);
            ui.add_sized(
                [138.0, 23.0],
                egui::Label::new(
                    egui::RichText::new(valeur)
                        .size(20.0)
                        .monospace()
                        .strong()
                        .color(accent),
                )
                .truncate()
                .selectable(false),
            );
        });
}

fn dessiner_graphique_crash(ui: &mut egui::Ui, rect: egui::Rect, jeu: &JeuCrash) {
    let time = ui.ctx().input(|i| i.time) as f32;
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 14.0, egui::Color32::from_rgb(9, 18, 31));
    painter.rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(1.4, egui::Color32::from_rgb(67, 100, 133)),
        egui::StrokeKind::Outside,
    );

    let zone = rect.shrink2(egui::vec2(18.0, 18.0));

    // Etoiles animées pour l'ambiance (plus denses)
    for i in 0..56 {
        let x_phase = ((time * 0.04 + i as f32 * 0.113) % 1.0).abs();
        let y_phase = ((i as f32 * 0.251 + 0.19) % 1.0).abs();
        let alpha = (45.0 + ((time * 3.0 + i as f32).sin() * 40.0 + 40.0)) as u8;
        let rayon = 1.1 + ((i as f32 * 0.3).sin().abs() * 0.9);
        painter.circle_filled(
            egui::pos2(
                zone.left() + x_phase * zone.width(),
                zone.top() + y_phase * zone.height(),
            ),
            rayon,
            egui::Color32::from_rgba_premultiplied(187, 208, 255, alpha),
        );
    }

    painter.line_segment(
        [
            egui::pos2(zone.left(), zone.bottom()),
            egui::pos2(zone.right(), zone.bottom()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(93, 109, 126)),
    );
    painter.line_segment(
        [
            egui::pos2(zone.left(), zone.bottom()),
            egui::pos2(zone.left(), zone.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(93, 109, 126)),
    );

    // Départ à gauche puis course sur toute la largeur.
    let x_depart = zone.left() + 10.0;
    let largeur_course = (zone.right() - x_depart - 8.0).max(12.0);
    let mult_pour_trace = if jeu.manche_en_cours() {
        jeu.multiplicateur_vol()
    } else {
        1.0
    };
    let point_fin = jeu
        .point_crash_revele()
        .unwrap_or(jeu.multiplicateur_vol().max(2.0))
        .max(2.0);
    let ymax = point_fin.max(5.0) as f32;
    let progression = ((mult_pour_trace as f32 - 1.0) / (ymax - 1.0)).clamp(0.0, 1.0);
    let x_courant = x_depart + progression * largeur_course;

    let mut points: Vec<egui::Pos2> = Vec::new();
    let n = ((progression * 140.0).ceil() as usize).max(2);
    for i in 0..n {
        let t_local = i as f32 / (n - 1) as f32;
        let t_global = progression * t_local;
        let x = x_depart + t_local * (x_courant - x_depart);
        let wobble = if jeu.est_en_vol() {
            (time * 6.0 + t_global * 12.0).sin() * 0.015 * t_global
        } else {
            0.0
        };
        let y = zone.bottom() - (t_global.powf(1.6) + wobble) * zone.height() * 0.86;
        points.push(egui::pos2(x, y));
    }

    if points.len() >= 2 {
        painter.add(egui::Shape::line(
            points.clone(),
            egui::Stroke::new(
                7.0,
                egui::Color32::from_rgba_premultiplied(46, 204, 113, 45),
            ),
        ));
        painter.add(egui::Shape::line(
            points.clone(),
            egui::Stroke::new(2.8, egui::Color32::from_rgb(89, 238, 146)),
        ));
    }

    if let Some(fin) = points.last().copied() {
        let size = if jeu.est_en_vol() {
            let pulse = 1.0 + 0.08 * (time * 10.0).sin().abs();
            egui::vec2(84.0, 84.0) * pulse
        } else {
            egui::vec2(80.0, 80.0)
        };
        let center = fin + egui::vec2(0.0, -6.0);
        let clamped_center = egui::pos2(
            center.x.clamp(
                zone.left() + size.x * 0.5 + 2.0,
                zone.right() - size.x * 0.5 - 2.0,
            ),
            center.y.clamp(
                zone.top() + size.y * 0.5 + 2.0,
                zone.bottom() - size.y * 0.5 - 2.0,
            ),
        );
        let img_rect = egui::Rect::from_center_size(clamped_center, size);
        ui.put(
            img_rect,
            egui::Image::new(egui::include_image!("../../../avion.png"))
                .fit_to_exact_size(img_rect.size()),
        );
    }

    if let Some(fin) = jeu.point_crash_revele() {
        let ratio = ((fin as f32 - 1.0) / (ymax - 1.0)).clamp(0.0, 1.0);
        let x = x_depart + ratio * largeur_course;
        painter.line_segment(
            [egui::pos2(x, zone.top()), egui::pos2(x, zone.bottom())],
            egui::Stroke::new(1.3, egui::Color32::from_rgb(231, 76, 60)),
        );
        painter.text(
            egui::pos2(x + 6.0, zone.top() + 8.0),
            egui::Align2::LEFT_TOP,
            format!("BOOM {:.2}x", fin),
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(231, 76, 60),
        );
    }

    painter.text(
        zone.left_top() + egui::vec2(6.0, 2.0),
        egui::Align2::LEFT_TOP,
        "x",
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(180, 190, 200),
    );
    painter.text(
        egui::pos2(x_depart, zone.bottom() + 2.0),
        egui::Align2::CENTER_TOP,
        "depart",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(160, 174, 192),
    );
    painter.text(
        zone.right_bottom() + egui::vec2(-8.0, -2.0),
        egui::Align2::RIGHT_BOTTOM,
        "temps",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(180, 190, 200),
    );
}

fn format_multiplicateur_historique(mult: f64) -> String {
    let mut base = format!("{:.4}", mult.max(0.0));
    while base.ends_with('0') {
        base.pop();
    }
    if base.ends_with('.') {
        base.push('0');
    }
    format!("{}x", base.replace('.', ","))
}
