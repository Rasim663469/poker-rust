use crate::core::cards::Carte;
use eframe::egui;

pub(super) fn dessiner_zone_label(painter: &egui::Painter, rect: egui::Rect, texte: &str) {
    painter.rect_filled(
        rect,
        12.0,
        egui::Color32::from_rgba_premultiplied(5, 20, 16, 170),
    );
    painter.rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(79, 124, 101)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        texte,
        egui::FontId::proportional(20.0),
        egui::Color32::from_rgb(220, 232, 227),
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
        egui::Color32::from_rgba_premultiplied(5, 20, 16, 170),
    );
    painter.rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(79, 124, 101)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{}  |  Stack: {}  |  Mise: {}", nom, jetons, mise_tour),
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(220, 232, 227),
    );
}

pub(super) fn dessiner_carte(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    card: Option<&Carte>,
    face_up: bool,
) {
    if face_up {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(249, 249, 245));
        painter.rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(74, 74, 80)),
            egui::StrokeKind::Outside,
        );
        if let Some(c) = card {
            let image_rect = rect.shrink(3.0);
            ui.put(
                image_rect,
                egui::Image::new(c.image_url_api()).fit_to_exact_size(image_rect.size()),
            );
            let txt = c.to_string();
            let red = txt.ends_with('C') || txt.ends_with('D');
            painter.text(
                rect.left_top() + egui::vec2(4.0, 3.0),
                egui::Align2::LEFT_TOP,
                txt,
                egui::FontId::proportional(13.0),
                if red {
                    egui::Color32::from_rgb(191, 39, 45)
                } else {
                    egui::Color32::from_rgb(22, 24, 28)
                },
            );
        }
    } else {
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(24, 47, 93));
        painter.rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(112, 148, 220)),
            egui::StrokeKind::Outside,
        );
        painter.rect_filled(rect.shrink(8.0), 6.0, egui::Color32::from_rgb(38, 62, 111));
    }
}

pub(super) fn dessiner_jetons(painter: &egui::Painter, center: egui::Pos2, n: usize) {
    for i in 0..n {
        let y = center.y - i as f32 * 6.0;
        let c = egui::pos2(center.x, y);
        painter.circle_filled(c, 12.0, egui::Color32::from_rgb(215, 56, 63));
        painter.circle_stroke(c, 12.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
        painter.circle_stroke(c, 7.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
    }
}
