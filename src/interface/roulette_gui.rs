use eframe::egui;
use crate::games::roulette::{Roulette, RouletteResult, RouletteColor, EUROPEAN_WHEEL_ORDER, european_color_for_number};
use crate::games::roulette::engine::{gain_multiplier, RouletteBet};

pub struct RouletteGuiState {
    pub last_result: Option<RouletteResult>,
    pub bet: Bet,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bet {
    None,
    Number(u8),
    Color(RouletteColor),
}

impl Default for Bet {
    fn default() -> Self { Bet::None }
}

impl Default for RouletteGuiState {
    fn default() -> Self {
        Self {
            last_result: None,
            bet: Bet::None,
            message: String::new(),
        }
    }
}

pub fn ui_roulette(state: &mut RouletteGuiState, ui: &mut egui::Ui) {
    ui.heading("Roulette");
    ui.add_space(10.0);
    // Dessin de la roue
    let (rect, _) = ui.allocate_exact_size(egui::vec2(340.0, 340.0), egui::Sense::hover());
    dessiner_roulette(ui, rect, state.last_result.as_ref().map(|r| r.number));
    ui.add_space(10.0);
    ui.label("Tableau des mises :");
    dessiner_tableau_mises(ui, &mut state.bet);

    // Affichage de la mise
    match state.bet {
        Bet::Number(n) => {
            ui.label(format!("Mise sur le numéro : {}", n));
        }
        Bet::Color(c) => {
            let (txt, col) = match c {
                RouletteColor::Red => ("Rouge", egui::Color32::from_rgb(200,0,0)),
                RouletteColor::Black => ("Noir", egui::Color32::from_rgb(30,30,30)),
                RouletteColor::Green => ("Vert", egui::Color32::from_rgb(0,180,0)),
            };
            ui.colored_label(col, format!("Mise sur la couleur : {}", txt));
        }
        Bet::None => {
            ui.label("Aucune mise sélectionnée");
        }
    }
    ui.add_space(10.0);
    if ui.button("Lancer la roue !").clicked() {
        let result = Roulette::spin();
        let bet_conv = bet_to_roulette_bet(&state.bet);
        let mult = gain_multiplier(bet_conv, &result);
        let win = mult > 0;
        state.last_result = Some(RouletteResult { win, ..result });
        state.message = if win {
            format!("Gagné ! (x{})", mult)
        } else {
            "Perdu...".to_string()
        };
    }
    if let Some(res) = &state.last_result {
        ui.add_space(10.0);
        ui.label(format!("Résultat : {} ({:?})", res.number, res.color));
        ui.label(&state.message);
    }
}

/// Affiche le tableau des mises interactif façon roulette européenne
fn dessiner_tableau_mises(ui: &mut egui::Ui, bet: &mut Bet) {
    use egui::Color32;
    // Tableau principal (3 lignes de 12, 0 à gauche)
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new("0").fill(Color32::from_rgb(0,180,0)).min_size(egui::vec2(32.0, 48.0))).clicked() {
            *bet = Bet::Number(0);
        }
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
                        let txt = n.to_string();
                        let mut btn = egui::Button::new(txt).fill(color).min_size(egui::vec2(32.0, 32.0));
                        if ui.add(btn).clicked() {
                            *bet = Bet::Number(n as u8);
                        }
                    }
                });
            }
            // Boutons colonnes (vertical, sous chaque colonne)
            ui.horizontal(|ui| {
                for col in 0..12 {
                    let col_label = match col {
                        0 => "2to1",
                        1 => "2to1",
                        2 => "2to1",
                        _ => "",
                    };
                    if col < 3 {
                        if ui.add(egui::Button::new(col_label).min_size(egui::vec2(32.0, 20.0))).clicked() {
                            *bet = Bet::Number(100+col as u8);
                        }
                    } else {
                        ui.add(egui::Label::new(" "));
                    }
                }
            });
        });
    });
    ui.add_space(4.0);
    // Douzaines
    ui.horizontal(|ui| {
        for (i, label) in ["1st 12", "2nd 12", "3rd 12"].iter().enumerate() {
            if ui.add(egui::Button::new(*label).min_size(egui::vec2(80.0, 24.0))).clicked() {
                *bet = Bet::Number(200 + i as u8);
            }
        }
    });
    // Pair/Impair, Rouge/Noir, Manque/Passe
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new("Pair").min_size(egui::vec2(60.0, 24.0))).clicked() {
            *bet = Bet::Number(210);
        }
        if ui.add(egui::Button::new("Impair").min_size(egui::vec2(60.0, 24.0))).clicked() {
            *bet = Bet::Number(211);
        }
        if ui.add(egui::Button::new("Rouge").fill(Color32::from_rgb(200,0,0)).min_size(egui::vec2(60.0, 24.0))).clicked() {
            *bet = Bet::Color(RouletteColor::Red);
        }
        if ui.add(egui::Button::new("Noir").fill(Color32::from_rgb(30,30,30)).min_size(egui::vec2(60.0, 24.0))).clicked() {
            *bet = Bet::Color(RouletteColor::Black);
        }
        if ui.add(egui::Button::new("Manque (1-18)").min_size(egui::vec2(90.0, 24.0))).clicked() {
            *bet = Bet::Number(212);
        }
        if ui.add(egui::Button::new("Passe (19-36)").min_size(egui::vec2(90.0, 24.0))).clicked() {
            *bet = Bet::Number(213);
        }
        if ui.add(egui::Button::new("Vert (0)").fill(Color32::from_rgb(0,180,0)).min_size(egui::vec2(60.0, 24.0))).clicked() {
            *bet = Bet::Color(RouletteColor::Green);
        }
    });
}

/// Conversion Bet (UI) -> RouletteBet (engine)
fn bet_to_roulette_bet(bet: &Bet) -> RouletteBet {
        match bet {
            Bet::Number(n) => {
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
            Bet::Color(c) => RouletteBet::Color(*c),
            Bet::None => RouletteBet::None,
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