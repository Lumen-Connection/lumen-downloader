use std::path::PathBuf;

use crate::app::App;
use crate::ui::i18n::Lang;
use crate::ui::theme;

const IMG_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];
const THUMB: f32 = 150.0;
const MAX_IMAGES: usize = 200;
const LOAD_PER_FRAME: usize = 6;

pub fn render(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let pt = app.config.lang == Lang::Pt;

    ui.label(
        egui::RichText::new(if pt { "Galeria de miniaturas" } else { "Thumbnail gallery" })
            .color(theme::text())
            .size(30.0)
            .strong(),
    );
    ui.label(
        egui::RichText::new(if pt {
            "Imagens da sua pasta de download."
        } else {
            "Images from your download folder."
        })
        .color(theme::text_muted())
        .size(14.0),
    );
    ui.add_space(16.0);

    // Coleta imagens (pasta de download + 1 nível de subpastas).
    let images = collect_images(&app.config.default_download_dir);

    if images.is_empty() {
        theme::card_frame().show(ui, |ui| {
            ui.label(
                egui::RichText::new(if pt {
                    "Nenhuma imagem encontrada na pasta de download."
                } else {
                    "No images found in the download folder."
                })
                .color(theme::text_faint()),
            );
        });
        return;
    }

    // Carrega no máximo algumas texturas por frame (evita travar a UI).
    let mut loaded_now = 0;
    for path in &images {
        if loaded_now >= LOAD_PER_FRAME {
            ctx.request_repaint();
            break;
        }
        if !app.gallery_textures.contains_key(path) {
            if let Some(tex) = load_thumb(ctx, path) {
                app.gallery_textures.insert(path.clone(), tex);
            } else {
                // Marca como tentado com uma textura vazia para não repetir.
                let blank = ctx.load_texture(
                    "blank",
                    egui::ColorImage::new([1, 1], theme::bg_card()),
                    egui::TextureOptions::LINEAR,
                );
                app.gallery_textures.insert(path.clone(), blank);
            }
            loaded_now += 1;
        }
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for path in &images {
                if let Some(tex) = app.gallery_textures.get(path) {
                    let [w, h] = tex.size();
                    let scale = THUMB / w.max(1) as f32;
                    let size = egui::vec2(THUMB, (h as f32 * scale).min(THUMB * 1.4));
                    let resp = ui
                        .add(egui::Image::from_texture((tex.id(), size)).sense(egui::Sense::click()))
                        .on_hover_text(
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        );
                    if resp.clicked() {
                        open::that(path).ok();
                    }
                } else {
                    // Placeholder enquanto carrega.
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(THUMB, THUMB), egui::Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        egui::Rounding::same(6.0),
                        theme::bg_card(),
                    );
                }
            }
        });
    });
}

fn collect_images(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let is_img = |p: &std::path::Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| IMG_EXTS.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false)
    };
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                // Apenas 1 nível de subpasta a partir da raiz.
                if dir == root {
                    dirs.push(p);
                }
            } else if is_img(&p) {
                out.push(p);
                if out.len() >= MAX_IMAGES {
                    return out;
                }
            }
        }
    }
    out
}

fn load_thumb(ctx: &egui::Context, path: &std::path::Path) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let img = img.thumbnail(300, 300);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    Some(ctx.load_texture(
        path.to_string_lossy(),
        color,
        egui::TextureOptions::LINEAR,
    ))
}
