use crate::games::mines::engine::{CaseMine, EtatMines, JeuMines};
use eframe::egui;

impl super::CasinoApp {
    pub(super) fn ui_mines(&mut self, ui: &mut egui::Ui) {
        let mut retour_menu = false;
        ui.horizontal(|ui| {
            if ui.button("<- Retour menu").clicked() {
                retour_menu = true;
            }
            ui.separator();
            ui.heading("Mines Royale");
            ui.separator();
            ui.label(
                egui::RichText::new("TABLE VIP")
                    .strong()
                    .color(egui::Color32::from_rgb(247, 211, 88)),
            );
        });
        if retour_menu {
            self.ecran = super::EcranCasino::Menu;
            self.mines = None;
            self.mines_paiement_applique = true;
            return;
        }

        if self.mines.is_none() {
            ui.separator();
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                ui_mines_indicateur(
                    ui,
                    "BALANCE",
                    format!("{:>8.2}", self.mines_solde),
                    egui::Color32::from_rgb(247, 211, 88),
                );
                ui_mines_indicateur(
                    ui,
                    "MISE SELECTIONNEE",
                    format!("{:>8.2}", self.mines_mise_input),
                    egui::Color32::from_rgb(93, 173, 226),
                );
                ui_mines_indicateur(
                    ui,
                    "NIVEAU DE RISQUE",
                    format!("{} mines", self.mines_nb_mines),
                    egui::Color32::from_rgb(231, 76, 60),
                );
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.heading("Configuration de la table");
                ui.add_space(6.0);

                ui.add(egui::Slider::new(&mut self.mines_nb_mines, 1..=24).text("Nombre de mines"));
                ui.horizontal_wrapped(|ui| {
                    ui.label("Presets mines:");
                    for preset in [3_u8, 5, 8, 12, 16, 20] {
                        if ui.small_button(format!("{}", preset)).clicked() {
                            self.mines_nb_mines = preset;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Mise:");
                    ui.add(
                        egui::DragValue::new(&mut self.mines_mise_input)
                            .range(0.01..=100_000.0)
                            .speed(1.0)
                            .max_decimals(2),
                    );
                });
                ui.horizontal(|ui| {
                    if ui.small_button("1/2").clicked() {
                        self.mines_mise_input = (self.mines_mise_input / 2.0).max(0.01);
                    }
                    if ui.small_button("x2").clicked() {
                        self.mines_mise_input = (self.mines_mise_input * 2.0).min(100_000.0);
                    }
                    if ui.small_button("MAX").clicked() {
                        self.mines_mise_input = self.mines_solde.clamp(0.01, 100_000.0);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Graine client:");
                    ui.text_edit_singleline(&mut self.mines_graine_client);
                });
                ui.label(format!("Nonce: {}", self.mines_nonce));
            });

            if !self.mines_ui_erreur.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(
                    egui::Color32::from_rgb(231, 76, 60),
                    egui::RichText::new(&self.mines_ui_erreur).strong(),
                );
            }

            ui.add_space(12.0);
            let btn = egui::Button::new(
                egui::RichText::new("LANCER LA SESSION")
                    .color(egui::Color32::from_rgb(24, 20, 9))
                    .size(24.0)
                    .strong(),
            )
            .fill(egui::Color32::from_rgb(247, 196, 84))
            .min_size(egui::vec2(340.0, 58.0));

            if ui.add(btn).clicked() {
                self.mines_ui_erreur.clear();

                if self.mines_mise_input > self.mines_solde {
                    self.mines_ui_erreur = format!(
                        "Solde insuffisant: balance {:.2}, mise {:.2}.",
                        self.mines_solde, self.mines_mise_input
                    );
                    return;
                }

                // Générer une graine serveur aléatoire
                let graine_serveur: String = (0..32)
                    .map(|_| format!("{:02x}", rand::random::<u8>()))
                    .collect();
                match JeuMines::nouveau(
                    self.mines_nb_mines,
                    self.mines_mise_input,
                    self.mines_graine_client.clone(),
                    graine_serveur,
                    self.mines_nonce,
                ) {
                    Ok(jeu) => {
                        self.mines_solde -= self.mines_mise_input;
                        self.mines_paiement_applique = false;
                        self.mines = Some(jeu);
                    }
                    Err(e) => {
                        self.mines_ui_erreur = e;
                    }
                }
            }
            return;
        }

        let mut est_termine = false;
        let mut fermer = false;

        {
            let jeu = self.mines.as_mut().unwrap();
            let actif = jeu.etat == EtatMines::Actif;
            let paiement_potentiel = jeu.mise * jeu.multiplicateur;
            let total_sures = (25_u8.saturating_sub(jeu.nb_mines)).max(1);
            let progression = (jeu.cases_revelees as f32 / total_sures as f32).clamp(0.0, 1.0);

            ui.separator();
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                ui_mines_indicateur(
                    ui,
                    "BALANCE",
                    format!("{:>8.2}", self.mines_solde),
                    egui::Color32::from_rgb(247, 211, 88),
                );
                ui_mines_indicateur(
                    ui,
                    "MISE ACTIVE",
                    format!("{:>8.2}", jeu.mise),
                    egui::Color32::from_rgb(93, 173, 226),
                );
                ui_mines_indicateur(
                    ui,
                    "MULTIPLICATEUR",
                    format!("{:>8.4}x", jeu.multiplicateur),
                    egui::Color32::from_rgb(46, 204, 113),
                );
                ui_mines_indicateur(
                    ui,
                    "CASH OUT",
                    format!("{:>8.2}", paiement_potentiel),
                    egui::Color32::from_rgb(245, 176, 65),
                );
                ui_mines_indicateur(
                    ui,
                    "CASES OUVERTES",
                    format!("{}/{}", jeu.cases_revelees, total_sures),
                    egui::Color32::from_rgb(171, 235, 198),
                );
            });

            ui.add_space(6.0);
            ui.add(
                egui::ProgressBar::new(progression)
                    .desired_width(ui.available_width())
                    .fill(egui::Color32::from_rgb(26, 148, 95))
                    .text(format!(
                        "Progression gemmes: {} / {}",
                        jeu.cases_revelees, total_sures
                    )),
            );

            let couleur_message = match jeu.etat {
                EtatMines::Perdu => egui::Color32::from_rgb(231, 76, 60),
                EtatMines::Gagne(_) => egui::Color32::from_rgb(46, 204, 113),
                _ => egui::Color32::from_rgb(226, 232, 240),
            };
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&jeu.message)
                    .size(19.0)
                    .strong()
                    .color(couleur_message),
            );

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("Hash serveur:")
                        .color(egui::Color32::from_rgb(148, 163, 184))
                        .strong(),
                );
                ui.monospace(format!("{}...", &jeu.hash_graine_serveur[..16]));
            });

            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                let btn_cashout = egui::Button::new(
                    egui::RichText::new(format!("CASH OUT {:.2}", paiement_potentiel))
                        .size(30.0)
                        .strong()
                        .color(egui::Color32::from_rgb(24, 20, 9)),
                )
                .fill(egui::Color32::from_rgb(247, 196, 84))
                .min_size(egui::vec2(420.0, 68.0));

                let peut_encaisser = actif && jeu.cases_revelees > 0;
                if ui.add_enabled(peut_encaisser, btn_cashout).clicked() {
                    let _ = jeu.encaisser();
                }
                if !peut_encaisser {
                    ui.label(
                        egui::RichText::new("Ouvre au moins une gemme pour débloquer le cash out.")
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );
                }

                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("AUTOPLAY")
                            .strong()
                            .color(egui::Color32::from_rgb(189, 195, 199)),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.mines_autoplay_count, 1..=20).text("Cases"),
                        );
                        for preset in [2_u8, 4, 6, 8] {
                            if ui.small_button(format!("{}", preset)).clicked() {
                                self.mines_autoplay_count = preset;
                            }
                        }
                        let btn_auto = egui::Button::new(
                            egui::RichText::new("LANCER")
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(egui::Color32::from_rgb(52, 73, 94));
                        if ui.add_enabled(actif, btn_auto).clicked() {
                            let n = self.mines_autoplay_count;
                            jeu.autoplay(n);
                        }
                    });
                });
            });

            ui.add_space(12.0);
            let gap = 8.0;
            let max_grid_w = ui.available_width().min(560.0);
            let cell_size = ((max_grid_w - gap * 4.0) / 5.0).clamp(54.0, 88.0);
            
            let total_grid_w = cell_size * 5.0 + gap * 4.0;
            let available = ui.available_width();
            let left_pad = ((available - total_grid_w) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(left_pad);
                ui.vertical(|ui| {
                    let mut case_cliquee: Option<(usize, usize)> = None;

                    egui::Grid::new("mines_grid")
                        .spacing(egui::vec2(gap, gap))
                        .min_col_width(cell_size)
                        .max_col_width(cell_size)
                        .show(ui, |ui| {
                            for ligne in 0..5 {
                                for col in 0..5 {
                                    let (rect, resp) = ui.allocate_exact_size(
                                        egui::vec2(cell_size, cell_size),
                                        if actif && jeu.grille[ligne][col] == CaseMine::Cachee {
                                            egui::Sense::click()
                                        } else {
                                            egui::Sense::hover()
                                        },
                                    );

                                    let case = jeu.grille[ligne][col];
                                    dessiner_case_mine(ui, rect, case, actif, ligne, col);

                                    if resp.clicked() {
                                        case_cliquee = Some((ligne, col));
                                    }
                                }
                                ui.end_row();
                            }
                        });

                    // Appliquer le clic
                    if let Some((l, c)) = case_cliquee {
                        let _ = jeu.reveler(l, c);
                    }
                });
            });

            est_termine = jeu.est_termine();
        }

        if est_termine && !self.mines_paiement_applique {
            if let Some(jeu) = self.mines.as_ref() {
                if let EtatMines::Gagne(p) = jeu.etat {
                    self.mines_solde += p;
                }
            }
            self.mines_paiement_applique = true;
        }

        if est_termine {
            let jeu = self.mines.as_ref().unwrap();
            ui.add_space(8.0);
            ui.separator();

            match jeu.etat {
                EtatMines::Gagne(p) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(46, 204, 113),
                        egui::RichText::new(format!("GAIN CONFIRME: {:.2}", p))
                            .size(22.0)
                            .strong(),
                    );
                }
                EtatMines::Perdu => {
                    ui.colored_label(
                        egui::Color32::from_rgb(231, 76, 60),
                        egui::RichText::new(format!("PERDU: mise {:.2}", jeu.mise))
                            .size(22.0)
                            .strong(),
                    );
                }
                _ => {}
            }
            ui.colored_label(
                egui::Color32::from_rgb(247, 211, 88),
                egui::RichText::new(format!("Balance actuelle: {:.2}", self.mines_solde))
                    .size(20.0)
                    .strong(),
            );

            // Vérification d'équité
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Graine serveur:");
                ui.monospace(&jeu.graine_serveur);
            });
            ui.horizontal(|ui| {
                ui.label("Verification equite:");
                if jeu.verifier_equite() {
                    ui.colored_label(egui::Color32::from_rgb(46, 204, 113), "Verifiee");
                } else {
                    ui.colored_label(egui::Color32::RED, "Echec");
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Nouvelle partie")
                                .size(18.0)
                                .strong()
                                .color(egui::Color32::from_rgb(24, 20, 9)),
                        )
                        .fill(egui::Color32::from_rgb(247, 196, 84))
                        .min_size(egui::vec2(210.0, 44.0)),
                    )
                    .clicked()
                {
                    self.mines_nonce += 1;
                    fermer = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Retour menu")
                                .size(17.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(52, 73, 94))
                        .min_size(egui::vec2(180.0, 44.0)),
                    )
                    .clicked()
                {
                    self.ecran = super::EcranCasino::Menu;
                    fermer = true;
                }
            });
        }

        if fermer {
            self.mines = None;
            self.mines_paiement_applique = true;
        }
    }
}

fn ui_mines_indicateur(ui: &mut egui::Ui, titre: &str, valeur: String, accent: egui::Color32) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(19, 31, 45))
        .stroke(egui::Stroke::new(1.0, accent))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(180.0, 68.0));
            ui.set_max_height(68.0);
            ui.set_width(180.0);
            ui.add_sized(
                [166.0, 14.0],
                egui::Label::new(
                    egui::RichText::new(titre)
                        .size(12.0)
                        .strong()
                        .color(egui::Color32::from_rgb(160, 174, 192)),
                )
                .truncate()
                .selectable(false),
            );
            ui.add_space(2.0);
            ui.add_sized(
                [166.0, 28.0],
                egui::Label::new(
                    egui::RichText::new(valeur)
                        .size(24.0)
                        .monospace()
                        .strong()
                        .color(accent),
                )
                .truncate()
                .selectable(false),
            );
        });
}

fn dessiner_case_mine(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    case: CaseMine,
    actif: bool,
    ligne: usize,
    col: usize,
) {
    let mut render_rect = rect;

    // Animation au survol (agrandissement)
    if actif && case == CaseMine::Cachee {
        let is_hovered = ui.rect_contains_pointer(rect);
        // Utilisation du temps pour lisser l'animation du scale
        let hover_factor =
            ui.ctx()
                .animate_bool_with_time(ui.id().with(ligne).with(col), is_hovered, 0.15);
        let scale = 1.0 + 0.05 * hover_factor; // grossit de 5%
        render_rect = egui::Rect::from_center_size(rect.center(), rect.size() * scale);
    }

    let painter = ui.painter_at(render_rect);

    match case {
        CaseMine::Cachee => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(34, 47, 62));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(87, 101, 116)),
                egui::StrokeKind::Outside,
            );
            // Effet d'éclat interne léger
            painter.rect_filled(
                render_rect.shrink(4.0),
                8.0,
                egui::Color32::from_rgb(40, 55, 71),
            );
        }
        CaseMine::Revelee => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(11, 83, 69));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(3.0, egui::Color32::from_rgb(46, 204, 113)),
                egui::StrokeKind::Outside,
            );

            // Image Diamant
            let img_rect = render_rect.shrink(10.0);
            egui::Image::new(egui::include_image!("../../../diamond.png"))
                .fit_to_exact_size(img_rect.size())
                .paint_at(ui, img_rect);
        }
        CaseMine::MineRevelee => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(146, 43, 33));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(3.0, egui::Color32::from_rgb(231, 76, 60)),
                egui::StrokeKind::Outside,
            );

            // Image Bombe
            let img_rect = render_rect.shrink(10.0);
            egui::Image::new(egui::include_image!("../../../mines.png"))
                .fit_to_exact_size(img_rect.size())
                .paint_at(ui, img_rect);
        }
        CaseMine::MineMontree => {
            painter.rect_filled(render_rect, 12.0, egui::Color32::from_rgb(60, 20, 20));
            painter.rect_stroke(
                render_rect,
                12.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(150, 50, 50)),
                egui::StrokeKind::Outside,
            );

            // Image Bombe transparente
            let img_rect = render_rect.shrink(12.0);
            egui::Image::new(egui::include_image!("../../../mines.png"))
                .fit_to_exact_size(img_rect.size())
                .tint(egui::Color32::from_white_alpha(100))
                .paint_at(ui, img_rect);
        }
    }
}
