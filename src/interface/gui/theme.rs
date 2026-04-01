use super::assets::{paint_contained_art, GameAsset};
use eframe::egui;

pub(super) const BG_DARK: egui::Color32 = egui::Color32::from_rgb(10, 13, 18);
pub(super) const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(17, 23, 31);
pub(super) const BG_PANEL_ALT: egui::Color32 = egui::Color32::from_rgb(22, 31, 40);
pub(super) const TABLE_GREEN: egui::Color32 = egui::Color32::from_rgb(18, 98, 68);
pub(super) const TABLE_GREEN_DEEP: egui::Color32 = egui::Color32::from_rgb(7, 53, 37);
pub(super) const GOLD: egui::Color32 = egui::Color32::from_rgb(212, 176, 84);
pub(super) const GOLD_SOFT: egui::Color32 = egui::Color32::from_rgb(240, 220, 158);
pub(super) const TEXT_MAIN: egui::Color32 = egui::Color32::from_rgb(238, 239, 234);
pub(super) const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(167, 178, 171);
pub(super) const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(176, 42, 51);

pub(super) fn apply_casino_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 10.0);
    style.spacing.menu_margin = egui::Margin::same(12);
    style.visuals.window_corner_radius = egui::CornerRadius::same(16);
    style.visuals.panel_fill = egui::Color32::TRANSPARENT;
    style.visuals.override_text_color = Some(TEXT_MAIN);
    style.visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, GOLD.gamma_multiply(0.35));
    style.visuals.widgets.inactive.bg_fill = BG_PANEL_ALT;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, GOLD.gamma_multiply(0.5));
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.2, TEXT_MAIN);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(34, 47, 58);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, GOLD);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.4, egui::Color32::WHITE);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(61, 44, 20);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.6, GOLD_SOFT);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(125, 92, 31);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, GOLD_SOFT);
    style.visuals.extreme_bg_color = BG_PANEL_ALT;
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(30.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(17.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );

    ctx.set_style(style);
}

pub(super) fn paint_global_background(
    ctx: &egui::Context,
    wallpaper: &egui::TextureHandle,
) {
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.image(
        wallpaper.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    painter.rect_filled(
        rect,
        0.0,
        egui::Color32::from_rgba_premultiplied(8, 12, 17, 95),
    );
}

pub(super) fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_PANEL)
        .corner_radius(egui::CornerRadius::same(18))
        .stroke(egui::Stroke::new(1.0, GOLD.gamma_multiply(0.4)))
        .inner_margin(egui::Margin::same(16))
}

pub(super) fn glass_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_premultiplied(20, 27, 36, 210))
        .corner_radius(egui::CornerRadius::same(22))
        .stroke(egui::Stroke::new(1.0, GOLD.gamma_multiply(0.28)))
        .inner_margin(egui::Margin::same(18))
}

pub(super) fn subpanel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_PANEL_ALT)
        .corner_radius(egui::CornerRadius::same(14))
        .stroke(egui::Stroke::new(1.0, GOLD.gamma_multiply(0.3)))
        .inner_margin(egui::Margin::same(14))
}

pub(super) fn info_card(ui: &mut egui::Ui, title: &str, body: &str) {
    subpanel_frame().show(ui, |ui| {
        ui.label(
            egui::RichText::new(title)
                .size(20.0)
                .strong()
                .color(GOLD_SOFT),
        );
        ui.add_space(6.0);
        ui.label(egui::RichText::new(body).color(TEXT_DIM));
    });
}

pub(super) fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(24.0)
            .strong()
            .color(GOLD_SOFT),
    );
    ui.label(egui::RichText::new(subtitle).color(TEXT_DIM));
}

pub(super) fn lobby_hero(
    ui: &mut egui::Ui,
    title: &str,
    slogan: &str,
    body: &str,
    cta_label: &str,
) -> egui::Response {
    let mut response = None;
    glass_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .size(18.0)
                    .strong()
                    .color(GOLD.gamma_multiply(0.95)),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(slogan)
                    .size(42.0)
                    .strong()
                    .color(GOLD_SOFT),
            );
            if !body.trim().is_empty() {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(body)
                        .size(17.0)
                        .color(TEXT_DIM),
                );
            }
            if !cta_label.trim().is_empty() {
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    response = Some(premium_button(ui, cta_label));
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Jeux premium, tables classiques et sessions rapides.")
                            .size(15.0)
                            .color(TEXT_DIM),
                    );
                });
            }
        });
    });
    response.unwrap_or_else(|| ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover()))
}

pub(super) fn premium_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .size(17.0)
                .strong()
                .color(BG_DARK),
        )
        .fill(GOLD_SOFT)
        .stroke(egui::Stroke::new(1.0, GOLD))
        .corner_radius(egui::CornerRadius::same(16))
        .min_size(egui::vec2(150.0, 44.0)),
    )
}

pub(super) fn back_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(TEXT_MAIN))
            .fill(BG_PANEL_ALT)
            .stroke(egui::Stroke::new(1.0, GOLD.gamma_multiply(0.35)))
            .corner_radius(egui::CornerRadius::same(14))
            .min_size(egui::vec2(148.0, 40.0)),
    )
}

pub(super) fn status_panel(ui: &mut egui::Ui, text: impl Into<String>) {
    subpanel_frame().show(ui, |ui| {
        ui.label(egui::RichText::new(text.into()).color(TEXT_DIM));
    });
}

pub(super) fn game_tile(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    meta: &str,
    accent: egui::Color32,
    asset: GameAsset,
) -> egui::Response {
    let desired = egui::vec2(360.0, 230.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let painter = ui.painter_at(rect);

    let fill = if response.hovered() {
        BG_PANEL_ALT
    } else {
        BG_PANEL
    };
    painter.rect_filled(rect, 22.0, fill);
    painter.rect_stroke(
        rect,
        22.0,
        egui::Stroke::new(if response.hovered() { 2.0 } else { 1.0 }, GOLD.gamma_multiply(0.7)),
        egui::StrokeKind::Outside,
    );

    let accent_rect = egui::Rect::from_min_size(rect.min, egui::vec2(10.0, rect.height()));
    painter.rect_filled(accent_rect, 22.0, accent);

    let meta_rect = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(26.0, 18.0),
        egui::vec2(102.0, 26.0),
    );
    painter.rect_filled(meta_rect, 12.0, accent.gamma_multiply(0.16));
    painter.rect_stroke(
        meta_rect,
        12.0,
        egui::Stroke::new(1.0, accent.gamma_multiply(0.85)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        meta_rect.center(),
        egui::Align2::CENTER_CENTER,
        meta,
        egui::FontId::proportional(13.0),
        GOLD_SOFT,
    );

    let text_left = rect.left() + 26.0;
    let text_right = rect.right() - 182.0;
    let title_pos = egui::pos2(text_left, rect.top() + 56.0);
    let subtitle_pos = egui::pos2(text_left, rect.top() + 118.0);

    let title_galley = painter.layout(
        title.to_owned(),
        egui::FontId::proportional(23.0),
        GOLD_SOFT,
        (text_right - text_left).max(120.0),
    );
    painter.galley(title_pos, title_galley, GOLD_SOFT);

    let subtitle_galley = painter.layout(
        subtitle.to_owned(),
        egui::FontId::proportional(15.0),
        TEXT_DIM,
        (text_right - text_left).max(120.0),
    );
    painter.galley(subtitle_pos, subtitle_galley, TEXT_DIM);

    let art_size = egui::vec2(136.0, 96.0);
    let image_zone_left = text_right + 18.0;
    let image_zone_right = rect.right() - 20.0;
    let art_center = egui::pos2(
        (image_zone_left + image_zone_right) * 0.5,
        rect.center().y,
    );
    let art_rect = egui::Rect::from_center_size(art_center, art_size);
    let inner = art_rect.shrink2(egui::vec2(2.0, 2.0));
    painter.rect_filled(
        inner.expand(4.0),
        16.0,
        egui::Color32::from_rgba_premultiplied(0, 0, 0, if response.hovered() { 42 } else { 30 }),
    );
    paint_contained_art(ui, inner, asset, 16);

    response
}
