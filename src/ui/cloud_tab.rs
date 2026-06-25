use crate::app::App;
use crate::ui::i18n::Lang;
use crate::ui::theme;

pub fn render(app: &mut App, _ctx: &egui::Context, ui: &mut egui::Ui) {
    let s = crate::ui::i18n::s(app.config.lang);
    let pt = app.config.lang == Lang::Pt;

    ui.label(
        egui::RichText::new(s.nav_cloud)
            .color(theme::text())
            .size(30.0)
            .strong(),
    );
    ui.label(
        egui::RichText::new(if pt {
            "Cópia automática dos downloads para uma pasta sincronizada (Drive, OneDrive, Dropbox…)."
        } else {
            "Automatically copy downloads to a synced folder (Drive, OneDrive, Dropbox…)."
        })
        .color(theme::text_muted())
        .size(14.0),
    );
    ui.add_space(20.0);

    let mut changed = false;

    theme::card_frame().show(ui, |ui| {
        let mut enabled = app.config.copy_to_cloud;
        if ui
            .checkbox(
                &mut enabled,
                if pt {
                    "Enviar cópia para a nuvem ao concluir"
                } else {
                    "Send a copy to the cloud when finished"
                },
            )
            .changed()
        {
            app.config.copy_to_cloud = enabled;
            changed = true;
        }

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(if pt { "Pasta da nuvem:" } else { "Cloud folder:" })
                .color(theme::text_muted())
                .size(12.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let mut path = app.config.cloud_folder.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut path)
                        .desired_width(ui.available_width() - 130.0)
                        .hint_text(if pt {
                            "Ex.: C:\\Users\\você\\Google Drive\\Lumen"
                        } else {
                            "e.g. C:\\Users\\you\\Google Drive\\Lumen"
                        })
                        .text_color(theme::text()),
                )
                .changed()
            {
                app.config.cloud_folder = path;
                changed = true;
            }
            if ui
                .add(theme::accent_button(if pt { "📁 Escolher" } else { "📁 Choose" }))
                .clicked()
            {
                if let Some(picked) = rfd::FileDialog::new().pick_folder() {
                    app.config.cloud_folder = picked.to_string_lossy().to_string();
                    changed = true;
                }
            }
        });
    });

    ui.add_space(12.0);
    theme::card_frame().show(ui, |ui| {
        ui.label(
            egui::RichText::new(if pt { "Como funciona" } else { "How it works" })
                .color(theme::text())
                .size(14.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(if pt {
                "Aponte para a pasta local que seu app do Drive/OneDrive/Dropbox sincroniza. \
                 Cada download concluído é copiado para lá, e o próprio serviço faz o upload. \
                 Não é preciso login dentro do Lumen."
            } else {
                "Point to the local folder your Drive/OneDrive/Dropbox app syncs. \
                 Each finished download is copied there, and the service uploads it. \
                 No login needed inside Lumen."
            })
            .color(theme::text_muted())
            .size(12.0),
        );
    });

    if app.config.copy_to_cloud && app.config.cloud_folder.trim().is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(if pt {
                "⚠ Defina a pasta da nuvem para a cópia funcionar."
            } else {
                "⚠ Set the cloud folder for copying to work."
            })
            .color(theme::danger())
            .size(12.0),
        );
    }

    if changed {
        app.config.save();
    }
}
