use crate::core::cards::Carte;
use eframe::egui;
use super::theme::{ACCENT_RED, GOLD, GOLD_SOFT, TABLE_GREEN_DEEP, TEXT_MAIN};

pub(super) fn dessiner_zone_label(painter: &egui::Painter, rect: egui::Rect, texte: &str) {
    painter.rect_filled(
        rect,
        12.0,
        egui::Color32::from_rgba_premultiplied(8, 19, 23, 210),
    );
    painter.rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.2, GOLD.gamma_multiply(0.8)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        texte,
        egui::FontId::proportional(19.0),
        GOLD_SOFT,
    );
}

pub(super) fn dessiner_joueur_zone(
    painter: &egui::Painter,
    rect: egui::Rect,
    nom: &str,
    jetons: u32,
    mise_tour: u32,
) {
    painter.rect_filled(
        rect,
        12.0,
        egui::Color32::from_rgba_premultiplied(8, 19, 23, 210),
    );
    painter.rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.2, GOLD.gamma_multiply(0.75)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{}  |  Stack: {}  |  Mise: {}", nom, jetons, mise_tour),
        egui::FontId::proportional(16.0),
        TEXT_MAIN,
    );
}

fn draw_card_image(ui: &mut egui::Ui, painter: &egui::Painter, rect: egui::Rect, card: &Carte) {
    // Si l'API fournit déjà une vraie image de carte, on l'affiche seule.
    // Ça évite le double rendu image + texte qui rendait les cartes illisibles.
    painter.rect_filled(rect, 10.0, egui::Color32::from_rgb(249, 247, 240));
    painter.rect_stroke(
        rect,
        10.0,
        egui::Stroke::new(1.2, egui::Color32::from_rgb(96, 83, 49)),
        egui::StrokeKind::Outside,
    );

    let image_rect = rect.shrink(3.0);
    ui.put(
        image_rect,
        egui::Image::new(card.image_url_api()).fit_to_exact_size(image_rect.size()),
    );
}

fn draw_card_fallback(painter: &egui::Painter, rect: egui::Rect, card: Option<&Carte>, face_up: bool) {
    // Ce fallback sert quand on n'a pas d'image complète disponible.
    // On garde ainsi un rendu propre même sans asset externe.
    if face_up {
        painter.rect_filled(rect, 10.0, egui::Color32::from_rgb(249, 247, 240));
        painter.rect_stroke(
            rect,
            10.0,
            egui::Stroke::new(1.2, egui::Color32::from_rgb(96, 83, 49)),
            egui::StrokeKind::Outside,
        );

        if let Some(c) = card {
            let txt = c.to_string();
            let red = txt.ends_with('C') || txt.ends_with('D');
            painter.text(
                rect.left_top() + egui::vec2(8.0, 7.0),
                egui::Align2::LEFT_TOP,
                &txt,
                egui::FontId::proportional(16.0),
                if red {
                    ACCENT_RED
                } else {
                    egui::Color32::from_rgb(22, 24, 28)
                },
            );
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                txt,
                egui::FontId::proportional(24.0),
                if red {
                    ACCENT_RED
                } else {
                    egui::Color32::from_rgb(22, 24, 28)
                },
            );
        }
    } else {
        painter.rect_filled(rect, 10.0, TABLE_GREEN_DEEP);
        painter.rect_stroke(
            rect,
            10.0,
            egui::Stroke::new(1.2, GOLD.gamma_multiply(0.9)),
            egui::StrokeKind::Outside,
        );
        painter.rect_filled(rect.shrink(8.0), 7.0, egui::Color32::from_rgb(17, 83, 66));
        painter.rect_stroke(
            rect.shrink(14.0),
            6.0,
            egui::Stroke::new(1.0, GOLD.gamma_multiply(0.4)),
            egui::StrokeKind::Outside,
        );
    }
}

pub(super) fn dessiner_carte(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    card: Option<&Carte>,
    face_up: bool,
) {
    // Ici on choisit explicitement un seul mode de rendu :
    // image complète OU fallback dessiné, jamais les deux.
    if face_up {
        if let Some(card) = card {
            draw_card_image(ui, painter, rect, card);
        } else {
            draw_card_fallback(painter, rect, None, true);
        }
    } else {
        draw_card_fallback(painter, rect, card, false);
    }
}

pub(super) fn dessiner_jetons(painter: &egui::Painter, center: egui::Pos2, n: usize) {
    for i in 0..n {
        let y = center.y - i as f32 * 6.0;
        let c = egui::pos2(center.x, y);
        painter.circle_filled(c, 12.0, ACCENT_RED);
        painter.circle_stroke(c, 12.0, egui::Stroke::new(1.5, GOLD_SOFT));
        painter.circle_stroke(c, 7.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
    }
}
