use crate::app::{App, DownloadPhase};
use crate::games::{self, GameTarget};
use crate::ui::i18n::Lang;
use crate::ui::theme;

pub fn render(app: &mut App, _ctx: &egui::Context, ui: &mut egui::Ui) {
    let s = crate::ui::i18n::s(app.config.lang);
    let pt = app.config.lang == Lang::Pt;

    ui.label(
        egui::RichText::new(s.nav_games)
            .color(theme::text())
            .size(30.0)
            .strong(),
    );
    ui.label(
        egui::RichText::new(if pt {
            "Insere as músicas baixadas na pasta que o jogo lê como rádio personalizado."
        } else {
            "Drops your downloaded music into the folder the game reads as a custom radio."
        })
        .color(theme::text_muted())
        .size(14.0),
    );
    ui.add_space(20.0);

    match app.selected_game {
        None => render_game_grid(app, ui, pt),
        Some(GameTarget::GtaV) => render_gtav(app, ui, pt),
    }
}

fn render_game_grid(app: &mut App, ui: &mut egui::Ui, pt: bool) {
    theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🎮").size(30.0));
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("GTA V")
                        .color(theme::text())
                        .size(18.0)
                        .strong(),
                );
                let dir = games::gtav_user_music_dir();
                let exists = dir.as_ref().map(|d| d.exists()).unwrap_or(false);
                let path_txt = dir
                    .map(|d| d.to_string_lossy().to_string())
                    .unwrap_or_else(|| "—".to_string());
                ui.label(
                    egui::RichText::new(path_txt)
                        .color(theme::text_muted())
                        .size(11.0),
                );
                ui.label(
                    egui::RichText::new(if exists {
                        if pt { "Pasta encontrada." } else { "Folder found." }
                    } else if pt {
                        "Pasta ainda não existe — será criada ao sincronizar."
                    } else {
                        "Folder doesn't exist yet — it'll be created on sync."
                    })
                    .color(if exists { theme::accent() } else { theme::text_muted() })
                    .size(11.0),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(theme::accent_button(if pt { "Abrir" } else { "Open" }))
                    .clicked()
                {
                    app.selected_game = Some(GameTarget::GtaV);
                }
            });
        });
    });
}

fn render_gtav(app: &mut App, ui: &mut egui::Ui, pt: bool) {
    if ui
        .add(theme::ghost_button(if pt { "← Jogos" } else { "← Games" }))
        .clicked()
    {
        app.selected_game = None;
        return;
    }
    ui.add_space(8.0);

    let history = app.db.get_history("music", app.config.max_history);

    if history.is_empty() {
        theme::card_frame().show(ui, |ui| {
            ui.label(
                egui::RichText::new(if pt {
                    "Nenhuma música baixada ainda. Baixe músicas na aba \"Baixar Música\"."
                } else {
                    "No downloaded music yet. Grab some in the \"Download Music\" tab."
                })
                .color(theme::text_muted())
                .size(13.0),
            );
        });
        return;
    }

    // Seleção rápida.
    ui.horizontal(|ui| {
        if ui
            .add(theme::ghost_button(if pt { "Selecionar todas" } else { "Select all" }))
            .clicked()
        {
            for e in &history {
                app.game_sync_selected.insert(e.id);
            }
        }
        if ui
            .add(theme::ghost_button(if pt { "Limpar seleção" } else { "Clear" }))
            .clicked()
        {
            app.game_sync_selected.clear();
        }
        ui.label(
            egui::RichText::new(format!(
                "{} / {}",
                app.game_sync_selected.len(),
                history.len()
            ))
            .color(theme::text_muted())
            .size(12.0),
        );
    });
    ui.add_space(8.0);

    theme::card_frame().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(320.0)
            .show(ui, |ui| {
                for e in &history {
                    let mut checked = app.game_sync_selected.contains(&e.id);
                    let label = format!("{}  ·  {}", e.title, e.format.to_uppercase());
                    if ui.checkbox(&mut checked, label).changed() {
                        if checked {
                            app.game_sync_selected.insert(e.id);
                        } else {
                            app.game_sync_selected.remove(&e.id);
                        }
                    }
                }
            });
    });

    ui.add_space(10.0);

    // Aviso do passo manual dentro do jogo (não há como automatizar de fora).
    theme::card_frame().show(ui, |ui| {
        ui.label(
            egui::RichText::new(if pt { "Depois de sincronizar" } else { "After syncing" })
                .color(theme::text())
                .size(13.0)
                .strong(),
        );
        ui.label(
            egui::RichText::new(if pt {
                "No GTA V, vá em Configurações → Áudio e clique em \"Buscar Músicas do Usuário\" \
                 para o rádio reconhecer as faixas. Depois, escolha a estação \"Independence FM\"."
            } else {
                "In GTA V, open Settings → Audio and click \"Scan User Music\" so the radio picks \
                 up the tracks. Then tune in to the \"Independence FM\" station."
            })
            .color(theme::text_muted())
            .size(11.0),
        );
    });

    ui.add_space(10.0);

    let selected_count = app.game_sync_selected.len();
    let sync = ui
        .add_enabled(
            selected_count > 0,
            theme::accent_button(&format!(
                "🎮  {} ({})",
                if pt { "Sincronizar" } else { "Sync" },
                selected_count
            ))
            .min_size(egui::vec2(180.0, 40.0)),
        )
        .clicked();

    if sync {
        let files: Vec<std::path::PathBuf> = history
            .iter()
            .filter(|e| app.game_sync_selected.contains(&e.id))
            .map(|e| std::path::PathBuf::from(&e.file_path))
            .collect();
        sync_gtav_flow(app, files);
    }
}

/// Copia (ou converte p/ MP3) as músicas selecionadas para a pasta User Music do
/// GTA V, em background. Espelha o fluxo de conversão em lote do converter_tab.
fn sync_gtav_flow(app: &mut App, files: Vec<std::path::PathBuf>) {
    let pt = app.config.lang == Lang::Pt;

    let Some(dest) = games::gtav_user_music_dir() else {
        app.toast(
            if pt {
                "Não foi possível localizar a pasta de Documentos."
            } else {
                "Couldn't locate your Documents folder."
            },
            true,
        );
        return;
    };
    if let Err(e) = std::fs::create_dir_all(&dest) {
        app.toast(
            if pt {
                format!("Falha ao criar a pasta do GTA V: {}", e)
            } else {
                format!("Failed to create GTA V folder: {}", e)
            },
            true,
        );
        return;
    }

    let engine = app.engine.clone();
    let convert_engine = app.config.convert_engine;
    let op_state = app.operation.clone();
    let toast_q = app.toast_queue.clone();

    {
        let mut op = op_state.lock().unwrap();
        op.phase = DownloadPhase::Downloading(if pt {
            "Sincronizando com GTA V...".to_string()
        } else {
            "Syncing to GTA V...".to_string()
        });
        op.progress = None;
    }

    app.download_task = Some(tokio::spawn(async move {
        let Some(eng) = engine else {
            op_state.lock().unwrap().phase = DownloadPhase::Failed(if pt {
                "Motor ainda inicializando. Tente de novo em instantes.".to_string()
            } else {
                "Engine still starting up. Try again in a moment.".to_string()
            });
            return;
        };

        let total = files.len();
        let (mut copied, mut converted, mut failed) = (0usize, 0usize, 0usize);
        let mut last_written = String::new();

        for (i, file) in files.iter().enumerate() {
            {
                let mut op = op_state.lock().unwrap();
                op.phase = DownloadPhase::Downloading(if pt {
                    format!("Sincronizando {}/{}...", i + 1, total)
                } else {
                    format!("Syncing {}/{}...", i + 1, total)
                });
            }

            if !file.is_file() {
                failed += 1;
                continue;
            }
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "musica".to_string());

            if games::is_gtav_supported(file) {
                let name = file
                    .file_name()
                    .map(|n| n.to_os_string())
                    .unwrap_or_else(|| format!("{}.mp3", stem).into());
                let out = dest.join(name);
                if std::fs::copy(file, &out).is_ok() {
                    copied += 1;
                    last_written = out.to_string_lossy().to_string();
                } else {
                    failed += 1;
                }
            } else {
                let out = dest.join(format!("{}.mp3", stem));
                match eng
                    .convert_file(
                        &file.to_string_lossy(),
                        &out.to_string_lossy(),
                        "mp3",
                        "",
                        convert_engine,
                    )
                    .await
                {
                    Ok(p) => {
                        converted += 1;
                        last_written = p.to_string_lossy().to_string();
                    }
                    Err(_) => failed += 1,
                }
            }
        }

        let ok = copied + converted;
        let summary = if pt {
            format!(
                "GTA V: {} copiada(s), {} convertida(s){}",
                copied,
                converted,
                if failed > 0 { format!(", {} falhou(ram)", failed) } else { String::new() }
            )
        } else {
            format!(
                "GTA V: {} copied, {} converted{}",
                copied,
                converted,
                if failed > 0 { format!(", {} failed", failed) } else { String::new() }
            )
        };
        toast_q.lock().unwrap().push((summary, ok == 0));

        let mut op = op_state.lock().unwrap();
        if ok == 0 {
            op.phase = DownloadPhase::Failed(if pt {
                "Nenhuma música foi sincronizada.".to_string()
            } else {
                "No music was synced.".to_string()
            });
        } else {
            op.phase = DownloadPhase::Completed(last_written);
        }
    }));
}
