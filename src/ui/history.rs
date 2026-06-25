use crate::app::{App, MediaType};
use crate::db::database::HistoryEntry;
use crate::ui::theme;

/// Tabela de histórico reutilizável com ações por item (abrir, abrir pasta,
/// baixar de novo e excluir) e botão de limpar tudo.
///
/// * `media_type` — chave usada no banco ("music"/"video"/"convert").
/// * `first_col` — rótulo da primeira coluna ("Título" ou "Arquivo").
/// * `redownload_as` — `Some(_)` habilita "baixar de novo" (abas de download).
pub fn render(
    app: &mut App,
    ui: &mut egui::Ui,
    media_type: &str,
    first_col: &str,
    title: &str,
    history: &[HistoryEntry],
    redownload_as: Option<MediaType>,
) {
    let s = crate::ui::i18n::s(app.config.lang);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .color(theme::text())
                .size(18.0)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !history.is_empty() && ui.add(theme::ghost_button(s.hist_clear)).clicked() {
                app.db.clear_history(media_type);
            }
            if !history.is_empty()
                && ui
                    .add(theme::ghost_button("⬇ .zip"))
                    .on_hover_text(s.hist_export)
                    .clicked()
            {
                let files: Vec<std::path::PathBuf> = history
                    .iter()
                    .map(|e| std::path::PathBuf::from(&e.file_path))
                    .collect();
                app.export_zip(files);
            }
        });
    });
    ui.add_space(6.0);

    // Formatos distintos presentes no histórico (para o filtro).
    let mut formats: Vec<String> = history
        .iter()
        .map(|e| e.format.clone())
        .filter(|f| !f.is_empty())
        .collect();
    formats.sort();
    formats.dedup();

    // Busca/filtro + alternância lista/grade
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.history_search)
                .hint_text(s.hist_search)
                .desired_width(220.0)
                .text_color(theme::text()),
        );
        let all = if app.config.lang == crate::ui::i18n::Lang::Pt { "Todos" } else { "All" };
        egui::ComboBox::from_id_source(format!("fmt_filter_{}", media_type))
            .selected_text(if app.history_format_filter.is_empty() {
                all.to_string()
            } else {
                app.history_format_filter.clone()
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.history_format_filter, String::new(), all);
                for f in &formats {
                    ui.selectable_value(&mut app.history_format_filter, f.clone(), f);
                }
            });
        let label = if app.config.history_grid {
            s.view_list
        } else {
            s.view_grid
        };
        if ui.add(theme::ghost_button(label)).clicked() {
            app.config.history_grid = !app.config.history_grid;
            app.config.save();
        }
    });
    ui.add_space(8.0);

    // Filtra por título (case-insensitive) e por formato.
    let needle = app.history_search.trim().to_lowercase();
    let fmt_filter = app.history_format_filter.clone();
    let filtered: Vec<&HistoryEntry> = history
        .iter()
        .filter(|e| needle.is_empty() || e.title.to_lowercase().contains(&needle))
        .filter(|e| fmt_filter.is_empty() || e.format == fmt_filter)
        .collect();

    if filtered.is_empty() {
        theme::card_frame().show(ui, |ui| {
            ui.label(egui::RichText::new(s.hist_empty).color(theme::text_faint()));
        });
    } else if app.config.history_grid {
        // Visualização em grade (cartões).
        egui::ScrollArea::vertical()
            .id_source(format!("hist_grid_{}", media_type))
            .max_height(360.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for entry in &filtered {
                        let frame = egui::Frame::none()
                            .fill(theme::bg_card())
                            .stroke(egui::Stroke::new(1.0, theme::border()))
                            .rounding(egui::Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(10.0));
                        frame.show(ui, |ui| {
                            ui.set_width(150.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(crate::ui::music_tab::short_link(
                                        &entry.title,
                                    ))
                                    .color(theme::text())
                                    .size(12.0),
                                );
                                ui.label(
                                    egui::RichText::new(&entry.format)
                                        .color(theme::text_muted())
                                        .size(11.0),
                                );
                                ui.horizontal(|ui| {
                                    if icon_button(ui, "▶", "Abrir") {
                                        open::that(&entry.file_path).ok();
                                    }
                                    if let Some(mt) = redownload_as {
                                        if icon_button(ui, "⟳", "Baixar de novo") {
                                            app.start_url_download(entry.url.clone(), mt);
                                        }
                                    }
                                    if icon_button(ui, "✕", "Excluir") {
                                        app.db.delete_history(entry.id);
                                    }
                                });
                            });
                        });
                    }
                });
            });
    } else {
        theme::card_frame().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_source(format!("hist_scroll_{}", media_type))
                .max_height(340.0)
                .show(ui, |ui| {
                    egui::Grid::new(format!("history_{}", media_type))
                        .striped(true)
                        .num_columns(4)
                        .min_col_width(70.0)
                        .spacing(egui::vec2(14.0, 10.0))
                        .show(ui, |ui| {
                            for h in [first_col, "Formato", "Data", ""] {
                                ui.label(egui::RichText::new(h).color(theme::accent()).strong());
                            }
                            ui.end_row();

                            for entry in filtered {
                                ui.horizontal(|ui| {
                                    let is_video = matches!(
                                        entry.format.as_str(),
                                        "mp4" | "mkv" | "webm" | "avi" | "mov"
                                    );
                                    if is_video {
                                        app.request_thumb(&entry.file_path);
                                        if let Some(tex) = app.thumb_textures.get(&entry.file_path) {
                                            let [w, h] = tex.size();
                                            let tw = 56.0;
                                            let th = (tw * h as f32 / w.max(1) as f32).min(40.0);
                                            ui.add(egui::Image::from_texture((
                                                tex.id(),
                                                egui::vec2(tw, th),
                                            )));
                                        } else {
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(56.0, 32.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                rect,
                                                egui::Rounding::same(4.0),
                                                theme::bg_card_hover(),
                                            );
                                        }
                                        ui.add_space(6.0);
                                    }
                                    ui.label(
                                        egui::RichText::new(&entry.title).color(theme::text()),
                                    );
                                });
                                ui.label(
                                    egui::RichText::new(&entry.format).color(theme::text_muted()),
                                );
                                ui.label(
                                    egui::RichText::new(&entry.created_at)
                                        .color(theme::text_muted()),
                                );

                                ui.horizontal(|ui| {
                                    if icon_button(ui, "▶", "Abrir arquivo") {
                                        open::that(&entry.file_path).ok();
                                    }
                                    if icon_button(ui, "📁", "Abrir pasta") {
                                        if let Some(parent) =
                                            std::path::Path::new(&entry.file_path).parent()
                                        {
                                            open::that(parent).ok();
                                        }
                                    }
                                    if let Some(mt) = redownload_as {
                                        if icon_button(ui, "⟳", "Baixar de novo") {
                                            app.start_url_download(entry.url.clone(), mt);
                                        }
                                    }
                                    if icon_button(ui, "🛡", "Verificar integridade") {
                                        app.verify_file(entry.file_path.clone());
                                    }
                                    if icon_button(ui, "ℹ", "Ver metadados") {
                                        app.show_metadata(entry.file_path.clone());
                                    }
                                    if !entry.url.is_empty() && icon_button(ui, "🔳", "QR do link") {
                                        if let Some(tex) =
                                            crate::app::make_qr_texture(ui.ctx(), &entry.url)
                                        {
                                            app.qr_window = Some((entry.url.clone(), tex));
                                        }
                                    }
                                    if icon_button(ui, "📋", "Copiar info") {
                                        let info = format!("{}\n{}", entry.title, entry.url);
                                        theme::set_clipboard(info.trim());
                                        app.toast("📋 Copiado", false);
                                    }
                                    if icon_button(ui, "🔗", "Copiar caminho") {
                                        theme::set_clipboard(&entry.file_path);
                                        app.toast("🔗 Caminho copiado", false);
                                    }
                                    if icon_button(ui, "✕", "Excluir") {
                                        app.db.delete_history(entry.id);
                                    }
                                });
                                ui.end_row();
                            }
                        });
                });
        });
    }

    // --- Lixeira ---
    let trash = app.db.get_deleted_history(media_type, app.config.max_history);
    if !trash.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("{} ({})", s.trash, trash.len()))
                .color(theme::text_muted()),
        )
        .id_source(format!("trash_{}", media_type))
        .show(ui, |ui| {
            if ui.add(theme::ghost_button(s.trash_empty_btn)).clicked() {
                app.db.empty_trash(media_type);
            }
            ui.add_space(6.0);
            for entry in &trash {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        egui::vec2(360.0, 18.0),
                        egui::Label::new(
                            egui::RichText::new(&entry.title)
                                .color(theme::text_faint())
                                .size(12.0),
                        ),
                    );
                    if icon_button(ui, "⟲", s.trash_restore) {
                        app.db.restore_history(entry.id);
                    }
                });
            }
        });
    }
}

fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> bool {
    ui.add(
        egui::Button::new(egui::RichText::new(icon).color(theme::text()))
            .fill(theme::bg_card())
            .min_size(egui::vec2(30.0, 26.0)),
    )
    .on_hover_text(tooltip)
    .clicked()
}
