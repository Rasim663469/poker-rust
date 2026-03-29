use crate::games::roulette::{Roulette, RouletteResult, RouletteColor, EUROPEAN_WHEEL_ORDER, european_color_for_number};
use crate::games::roulette::engine::{gain_multiplier, RouletteBet};
use eframe::egui;
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouletteBetUI {
    None,
    Number(u8),
    Color(RouletteColor),
}

impl Default for RouletteBetUI {
    fn default() -> Self {
        RouletteBetUI::None
    }
}

pub struct RouletteAnim {
    pub start_time: std::time::Instant,
    pub duration: std::time::Duration,
    pub final_number: u8,
    pub total_spins: usize,
}

impl super::CasinoApp {
    pub(super) fn ui_roulette(&mut self, ui: &mut egui::Ui) {
        // Slider de mise
        ui.horizontal(|ui| {
            ui.label("Mise:");
            if ui.add(egui::Slider::new(&mut self.roulette_mise, 1..=500).text("jetons")).changed() {
                self.roulette_last_result = None;  // Clear le résultat précédent quand on change la mise
            }
        });
        ui.add_space(10.0);

        // Bouton Lancer la roue (désactivé si aucune mise)
        if ui.add_enabled(self.roulette_bet != RouletteBetUI::None, egui::Button::new("Lancer la roue !")).clicked() && self.roulette_anim.is_none() {
            self.roulette_last_result = None;  // Clear le résultat précédent
            let result = Roulette::spin();
            let mut rng = rand::thread_rng();
            let spins = rng.gen_range(3..=5);
            let duration = std::time::Duration::from_millis(rng.gen_range(1800..=2500));
            self.roulette_anim = Some(RouletteAnim {
                start_time: std::time::Instant::now(),
                duration,
                final_number: result.number,
                total_spins: spins,
            });
        }

        // Gérer l'animation et afficher le résultat
        let mut highlight = None;
        if let Some(anim) = &self.roulette_anim {
            let elapsed = anim.start_time.elapsed().as_secs_f32();
            let total = anim.duration.as_secs_f32();
            let progress = (elapsed / total).min(1.0);
            let n_cases = 37;
            let spins = anim.total_spins as f32;
            let ease = |t: f32| -> f32 { 3.0*t.powi(2) - 2.0*t.powi(3) };
            let eased = ease(progress);
            let total_steps = spins * n_cases as f32 + EUROPEAN_WHEEL_ORDER.iter().position(|&n| n == anim.final_number).unwrap_or(0) as f32;
            let idx = ((1.0 - eased) * (spins * n_cases as f32) + eased * total_steps).round() as usize % n_cases;
            highlight = Some(EUROPEAN_WHEEL_ORDER[idx]);

            if progress >= 1.0 {
                // Animation terminée, afficher le résultat
                let result = RouletteResult {
                    number: anim.final_number,
                    color: european_color_for_number(anim.final_number),
                    win: {
                        let bet_conv = bet_ui_to_roulette_bet(&self.roulette_bet);
                        gain_multiplier(bet_conv, &RouletteResult { number: anim.final_number, color: european_color_for_number(anim.final_number), win: false }) > 0
                    },
                };
                self.roulette_last_result = Some(result);
                self.roulette_anim = None;
            } else {
                // Animation en cours, redemander repaint
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            }
        } else {
            highlight = self.roulette_last_result.as_ref().map(|r| r.number);
        }

        ui.add_space(10.0);

        // Fond ambiance casino
        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(12, 96, 66))  // Vert casino
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 100)))
            .corner_radius(10.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                // Layout horizontal : Roue à gauche, Tableau à droite
                ui.horizontal(|ui| {
                    ui.add_space(50.0);

                    ui.vertical(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(340.0, 340.0), egui::Sense::hover());
                        dessiner_roulette(ui, rect, highlight);
                    });

                    ui.add_space(160.0);

                    ui.vertical(|ui| {
                        ui.add_space(50.0);
                        ui.colored_label(egui::Color32::WHITE, egui::RichText::new("TABLEAU DES MISES :").heading().size(22.0));
                        dessiner_tableau_mises(ui, &mut self.roulette_bet, &mut self.roulette_last_result);
                        ui.add_space(50.0);
                    });
                });
            });

        ui.add_space(10.0);

        // Affichage de la mise
        match self.roulette_bet {
            RouletteBetUI::Color(RouletteColor::Red) => {
                ui.colored_label(egui::Color32::from_rgb(200,0,0), egui::RichText::new(bet_ui_to_display_string(&self.roulette_bet)).heading());
            }
            RouletteBetUI::Color(RouletteColor::Black) => {
                ui.colored_label(egui::Color32::from_rgb(30,30,30), egui::RichText::new(bet_ui_to_display_string(&self.roulette_bet)).heading());
            }
            RouletteBetUI::Color(RouletteColor::Green) => {
                ui.colored_label(egui::Color32::from_rgb(0,180,0), egui::RichText::new(bet_ui_to_display_string(&self.roulette_bet)).heading());
            }
            _ => {
                ui.label(egui::RichText::new(bet_ui_to_display_string(&self.roulette_bet)).heading());
            }
        }

        // Affichage du résultat
        if let Some(result) = &self.roulette_last_result {
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label(egui::RichText::new(format!("Résultat : {}", result.number)).heading());

            let (txt_resultat, col_resultat) = if result.win {
                ("GAGNÉ", egui::Color32::from_rgb(0, 153, 255))  // Bleu
            } else {
                ("PERDU", egui::Color32::from_rgb(255, 153, 0))  // Orange
            };

            ui.colored_label(col_resultat, egui::RichText::new(txt_resultat).heading());

            // Affichage du multiplicateur et du gain
            let bet_conv = bet_ui_to_roulette_bet(&self.roulette_bet);
            let multiplier = gain_multiplier(bet_conv, result) as i32;
            let gain = (self.roulette_mise as i32) * multiplier;

            ui.add_space(5.0);
            ui.colored_label(col_resultat, egui::RichText::new(format!("Multiplicateur : x{}", multiplier)).heading());
            ui.colored_label(col_resultat, egui::RichText::new(format!("Gain total : {} jetons", gain)).heading());
        }

        // Return to menu button (top-left, like blackjack and slotmachine)
        if ui.button("<- Retour menu").clicked() {
            self.ecran = super::EcranCasino::Menu;
        }
    }
}

/// Affiche le tableau des mises interactif façon roulette européenne
fn dessiner_tableau_mises(ui: &mut egui::Ui, bet: &mut RouletteBetUI, last_result: &mut Option<RouletteResult>) {
    use egui::Color32;

    let old_bet = *bet;

    // Horizontal: [0 vertical | grille 3x12 | colonne 2to1]
    ui.horizontal(|ui| {
        // Bouton 0 vertical (gauche)
        ui.vertical(|ui| {
            if ui.add(egui::Button::new(egui::RichText::new("0").size(24.0).color(Color32::WHITE)).fill(Color32::from_rgb(0,180,0)).min_size(egui::vec2(32.0, 100.0))).clicked() {
                *bet = RouletteBetUI::Number(0);
            }
        });

        // Grille 3x12 numérotée
        ui.vertical(|ui| {
            for row in 0..3 {
                ui.horizontal(|ui| {
                    for col in 0..12 {
                        let n = 3*col + (2-row) + 1;
                        if n > 36 { continue; }
                        let color = match european_color_for_number(n as u8) {
                            RouletteColor::Red => Color32::from_rgb(200,0,0),
                            RouletteColor::Black => Color32::from_rgb(30,30,30),
                            RouletteColor::Green => Color32::from_rgb(0,180,0),
                        };
                        if ui.add(egui::Button::new(egui::RichText::new(n.to_string()).color(Color32::WHITE)).fill(color).min_size(egui::vec2(32.0, 32.0))).clicked() {
                            *bet = RouletteBetUI::Number(n as u8);
                        }
                    }
                });
            }
        });

        // Colonne 2to1 (droite)
        ui.vertical(|ui| {
            for col in 0..3 {
                if ui.add(egui::Button::new(egui::RichText::new("2to1").color(Color32::BLACK)).fill(Color32::WHITE).min_size(egui::vec2(48.0, 32.0))).clicked() {
                    *bet = RouletteBetUI::Number(100+col as u8);
                }
            }
        });
    });

    ui.add_space(8.0);

    // Douzaines (alignées sur la largeur grille - sans le zéro)
    ui.horizontal(|ui| {
        ui.add_space(36.0);  // Largeur du 0 (32) + petite marge (4)
        for (i, label) in ["1st 12", "2nd 12", "3rd 12"].iter().enumerate() {
            if ui.add(egui::Button::new(egui::RichText::new(*label).color(Color32::BLACK)).fill(Color32::WHITE).min_size(egui::vec2(126.0, 24.0))).clicked() {
                *bet = RouletteBetUI::Number(200 + i as u8);
            }
        }
    });

    ui.add_space(4.0);

    // Boutons du bas (1-18, Pair, Rouge, Noir, Impair, 19-36)
    ui.horizontal(|ui| {
        ui.add_space(36.0);  // Largeur du 0 (32) + petite marge (4)
        if ui.add(egui::Button::new(egui::RichText::new("1-18").color(Color32::BLACK)).fill(Color32::WHITE).min_size(egui::vec2(63.0, 24.0))).clicked() {
            *bet = RouletteBetUI::Number(212);
        }
        if ui.add(egui::Button::new(egui::RichText::new("Pair").color(Color32::BLACK)).fill(Color32::WHITE).min_size(egui::vec2(63.0, 24.0))).clicked() {
            *bet = RouletteBetUI::Number(210);
        }
        if ui.add(egui::Button::new(egui::RichText::new("R").color(Color32::WHITE)).fill(Color32::from_rgb(200,0,0)).min_size(egui::vec2(63.0, 24.0))).clicked() {
            *bet = RouletteBetUI::Color(RouletteColor::Red);
        }
        if ui.add(egui::Button::new(egui::RichText::new("N").color(Color32::WHITE)).fill(Color32::from_rgb(30,30,30)).min_size(egui::vec2(63.0, 24.0))).clicked() {
            *bet = RouletteBetUI::Color(RouletteColor::Black);
        }
        if ui.add(egui::Button::new(egui::RichText::new("Impair").color(Color32::BLACK)).fill(Color32::WHITE).min_size(egui::vec2(63.0, 24.0))).clicked() {
            *bet = RouletteBetUI::Number(211);
        }
        if ui.add(egui::Button::new(egui::RichText::new("19-36").color(Color32::BLACK)).fill(Color32::WHITE).min_size(egui::vec2(63.0, 24.0))).clicked() {
            *bet = RouletteBetUI::Number(213);
        }
    });

    // Si la mise a changé, effacer le résultat précédent
    if *bet != old_bet {
        *last_result = None;
    }
}

/// Conversion RouletteBetUI (UI) -> RouletteBet (engine)
fn bet_ui_to_roulette_bet(bet: &RouletteBetUI) -> RouletteBet {
    match bet {
        RouletteBetUI::Number(n) => {
            // Cas spéciaux pour les boutons du tableau interactif
            match *n {
                100..=102 => RouletteBet::Column(*n - 100),
                200..=202 => RouletteBet::Dozen(*n - 200),
                210 => RouletteBet::Even,
                211 => RouletteBet::Odd,
                212 => RouletteBet::Low,
                213 => RouletteBet::High,
                n => RouletteBet::Number(n),
            }
        }
        RouletteBetUI::Color(c) => RouletteBet::Color(*c),
        RouletteBetUI::None => RouletteBet::None,
    }
}

/// Convertir RouletteBetUI en texte lisible pour l'affichage
fn bet_ui_to_display_string(bet: &RouletteBetUI) -> String {
    match bet {
        RouletteBetUI::Number(n) => {
            match *n {
                0 => "Mise sur le 0".to_string(),
                100 => "Mise sur colonne 1 (2to1)".to_string(),
                101 => "Mise sur colonne 2 (2to1)".to_string(),
                102 => "Mise sur colonne 3 (2to1)".to_string(),
                200 => "Mise sur 1st 12".to_string(),
                201 => "Mise sur 2nd 12".to_string(),
                202 => "Mise sur 3rd 12".to_string(),
                210 => "Mise sur Pair".to_string(),
                211 => "Mise sur Impair".to_string(),
                212 => "Mise sur 1-18".to_string(),
                213 => "Mise sur 19-36".to_string(),
                n => format!("Mise sur le numéro : {}", n),
            }
        }
        RouletteBetUI::Color(c) => {
            match c {
                RouletteColor::Red => "Mise sur Rouge".to_string(),
                RouletteColor::Black => "Mise sur Noir".to_string(),
                RouletteColor::Green => "Mise sur Vert".to_string(),
            }
        }
        RouletteBetUI::None => "Aucune mise sélectionnée".to_string(),
    }
}

fn dessiner_roulette(ui: &mut egui::Ui, rect: egui::Rect, highlight: Option<u8>) {
    use egui::{Color32, Painter};
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let rayon = rect.width().min(rect.height()) * 0.48;
    let n_cases = 37;
    let angle_par_case = std::f32::consts::TAU / n_cases as f32;
    for i in 0..n_cases {
        let angle_deb = i as f32 * angle_par_case - std::f32::consts::FRAC_PI_2;
        let angle_fin = (i + 1) as f32 * angle_par_case - std::f32::consts::FRAC_PI_2;
        let number = EUROPEAN_WHEEL_ORDER[i];
        let color = match european_color_for_number(number) {
            RouletteColor::Green => Color32::from_rgb(0, 180, 0),
            RouletteColor::Red => Color32::from_rgb(200, 0, 0),
            RouletteColor::Black => Color32::from_rgb(30, 30, 30),
        };
        let highlight_case = highlight == Some(number);
        let fill = if highlight_case {
            Color32::from_rgb(255, 220, 80)
        } else {
            color
        };
        // Points du secteur (triangle-fan)
        let mut points = vec![];
        let n_steps = 8;
        points.push(center + egui::vec2(angle_deb.cos(), angle_deb.sin()) * (rayon * 0.60));
        for step in 0..=n_steps {
            let t = step as f32 / n_steps as f32;
            let angle = angle_deb + t * (angle_fin - angle_deb);
            points.push(center + egui::vec2(angle.cos(), angle.sin()) * rayon);
        }
        points.push(center + egui::vec2(angle_fin.cos(), angle_fin.sin()) * (rayon * 0.60));
        painter.add(egui::Shape::convex_polygon(points, fill, egui::Stroke::new(2.0, Color32::BLACK)));
        // Numéro
        let angle_txt = (angle_deb + angle_fin) / 2.0;
        let pos = center + egui::vec2(angle_txt.cos(), angle_txt.sin()) * (rayon * 0.80);
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            number.to_string(),
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    }
    // Centre
    painter.circle_filled(center, rayon * 0.58, Color32::from_rgb(18, 96, 66));
    painter.circle_stroke(center, rayon * 0.58, egui::Stroke::new(2.0, Color32::WHITE));
    painter.text(center, egui::Align2::CENTER_CENTER, "ROULETTE", egui::FontId::proportional(22.0), Color32::WHITE);
    // Curseur/bille
    if let Some(num) = highlight {
        if let Some(idx) = EUROPEAN_WHEEL_ORDER.iter().position(|&n| n == num) {
            let angle = idx as f32 * angle_par_case - std::f32::consts::FRAC_PI_2 + angle_par_case/2.0;
            let pos = center + egui::vec2(angle.cos(), angle.sin()) * (rayon * 0.95);
            painter.circle_filled(pos, 10.0, Color32::YELLOW);
            painter.circle_stroke(pos, 10.0, egui::Stroke::new(2.0, Color32::BLACK));
        }
    }
}
