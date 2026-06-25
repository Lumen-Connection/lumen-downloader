use crate::app::{App, MediaType, Tab};
use crate::db::database::HistoryEntry;
use crate::ui::theme;

pub fn render(app: &mut App, _ctx: &egui::Context, ui: &mut egui::Ui) {
    let s = crate::ui::i18n::s(app.config.lang);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("◆").size(28.0).color(theme::accent()));
        ui.label(
            egui::RichText::new(s.home_title)
                .color(theme::text())
                .size(30.0)
                .strong(),
        );
    });
    ui.label(
        egui::RichText::new(s.home_subtitle)
            .color(theme::text_muted())
            .size(14.0),
    );
    ui.add_space(20.0);

    // Download rápido (vídeo).
    let mut submit = false;
    theme::card_frame().show(ui, |ui| {
        ui.label(
            egui::RichText::new(s.home_quick)
                .color(theme::text_muted())
                .size(11.0)
                .strong(),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let resp = ui.add_sized(
                egui::vec2(ui.available_width() - 140.0, 40.0),
                egui::TextEdit::singleline(&mut app.video_url)
                    .hint_text("https://...")
                    .text_color(theme::text())
                    .margin(egui::vec2(12.0, 10.0)),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            if ui
                .add(theme::accent_button(&format!("⤓  {}", s.download)).min_size(egui::vec2(120.0, 40.0)))
                .clicked()
            {
                submit = true;
            }
        });
    });
    if submit {
        let url = app.video_url.clone();
        app.start_url_download(url, MediaType::Video);
    }

    ui.add_space(18.0);

    // Atalhos para as abas.
    let mut nav: Option<Tab> = None;
    ui.horizontal_wrapped(|ui| {
        if shortcut(ui, &format!("🎵  {}", s.nav_music)) {
            nav = Some(Tab::Music);
        }
        if shortcut(ui, &format!("🎬  {}", s.nav_video)) {
            nav = Some(Tab::Video);
        }
        if shortcut(ui, s.transcribe) {
            nav = Some(Tab::Video);
        }
        if shortcut(ui, &format!("🔄  {}", s.nav_converter)) {
            nav = Some(Tab::Converter);
        }
    });
    if let Some(t) = nav {
        app.active_tab = t;
    }

    ui.add_space(20.0);

    // Recentes (todos os tipos).
    ui.label(
        egui::RichText::new(s.home_recents)
            .color(theme::text())
            .size(18.0)
            .strong(),
    );
    ui.add_space(10.0);

    let mut recents: Vec<HistoryEntry> = Vec::new();
    for mt in ["music", "video", "convert"] {
        recents.extend(app.db.get_history(mt, 10));
    }
    recents.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    recents.truncate(8);

    if recents.is_empty() {
        theme::card_frame().show(ui, |ui| {
            ui.label(egui::RichText::new(s.home_empty).color(theme::text_faint()));
        });
        return;
    }

    render_recents(ui, &recents);
}

/// Botão de atalho grande para uma aba. Retorna `true` se clicado.
fn shortcut(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(
        egui::Button::new(
            egui::RichText::new(label).color(theme::text()).size(15.0),
        )
        .fill(theme::bg_card())
        .rounding(egui::Rounding::same(10.0))
        .min_size(egui::vec2(175.0, 52.0)),
    )
    .clicked()
}

fn render_recents(ui: &mut egui::Ui, recents: &[HistoryEntry]) {
    theme::card_frame().show(ui, |ui| {
        for entry in recents {
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("▶").color(theme::text()))
                            .fill(theme::bg_card())
                            .min_size(egui::vec2(28.0, 24.0)),
                    )
                    .clicked()
                {
                    open::that(&entry.file_path).ok();
                }
                ui.label(
                    egui::RichText::new(crate::ui::music_tab::short_link(&entry.title))
                        .color(theme::text()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(&entry.created_at)
                            .color(theme::text_faint())
                            .size(11.0),
                    );
                });
            });
            ui.separator();
        }
    });
}
