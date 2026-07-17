use std::path::{Path, PathBuf};

use printpdf::{
    BuiltinFont, ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px,
};

use super::DownloadEngine;

impl DownloadEngine {
    async fn ensure_pdfium(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        use pdfium_render::prelude::Pdfium;

        let lib_name = Pdfium::pdfium_platform_library_name();
        let lib_path = self.libs_dir.join(&lib_name);
        if lib_path.exists() {
            return Ok(lib_path);
        }

        let asset = if cfg!(windows) {
            "pdfium-win-x64.tgz"
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "pdfium-mac-arm64.tgz"
            } else {
                "pdfium-mac-x64.tgz"
            }
        } else {
            "pdfium-linux-x64.tgz"
        };
        let url = format!(
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/{}",
            asset
        );

        let bytes = reqwest::get(&url).await?.error_for_status()?.bytes().await?;

        let lib_path2 = lib_path.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes[..]));
            let mut archive = tar::Archive::new(gz);
            for entry in archive.entries().map_err(|e| e.to_string())? {
                let mut entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path().map_err(|e| e.to_string())?.into_owned();
                if path.file_name() == Some(lib_name.as_os_str()) {
                    entry.unpack(&lib_path2).map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
            Err("pdfium não encontrado no pacote baixado".to_string())
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(lib_path)
    }

    pub(super) async fn pdf_to_images(
        &self,
        input: &Path,
        out_base: &Path,
        format: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let pdfium_path = self.ensure_pdfium().await?;

        let input = input.to_path_buf();
        let folder = out_base
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let stem = out_base
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "pagina".to_string());
        let ext = format.to_string();

        let first = tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
            use pdfium_render::prelude::*;

            let bindings = Pdfium::bind_to_library(&pdfium_path)
                .map_err(|e| format!("falha ao carregar pdfium: {}", e))?;
            let pdfium = Pdfium::new(bindings);
            let document = pdfium
                .load_pdf_from_file(&input, None)
                .map_err(|e| format!("falha ao abrir o PDF: {}", e))?;

            let config = PdfRenderConfig::new().scale_page_by_factor(2.0);
            let pages = document.pages();
            let total = pages.len();
            if total == 0 {
                return Err("o PDF não tem páginas".to_string());
            }

            let mut first_path: Option<PathBuf> = None;
            for (index, page) in pages.iter().enumerate() {
                let image = page
                    .render_with_config(&config)
                    .map_err(|e| e.to_string())?
                    .as_image();

                let name = if total == 1 {
                    format!("{}.{}", stem, ext)
                } else {
                    format!("{}_pagina_{}.{}", stem, index + 1, ext)
                };
                let path = folder.join(name);

                let result = if ext == "jpg" || ext == "jpeg" {
                    image.into_rgb8().save(&path)
                } else {
                    image.save(&path)
                };
                result.map_err(|e| e.to_string())?;

                if first_path.is_none() {
                    first_path = Some(path);
                }
            }
            Ok(first_path.unwrap())
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(first)
    }

    pub async fn merge_pdfs(
        &self,
        inputs: Vec<PathBuf>,
        out: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let pdfium_path = self.ensure_pdfium().await?;
        let out2 = out.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
            use pdfium_render::prelude::*;
            let bindings = Pdfium::bind_to_library(&pdfium_path).map_err(|e| e.to_string())?;
            let pdfium = Pdfium::new(bindings);
            let mut dest = pdfium.create_new_pdf().map_err(|e| e.to_string())?;
            for inp in &inputs {
                let src = pdfium
                    .load_pdf_from_file(inp, None)
                    .map_err(|e| format!("abrir {}: {}", inp.display(), e))?;
                let n = src.pages().len();
                if n == 0 {
                    continue;
                }
                let idx = dest.pages().len();
                dest.pages_mut()
                    .copy_pages_from_document(&src, &format!("1-{}", n), idx)
                    .map_err(|e| e.to_string())?;
            }
            dest.save_to_file(&out2).map_err(|e| e.to_string())?;
            Ok(out2)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.into())
    }

    pub async fn split_pdf(
        &self,
        input: &Path,
        out_folder: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let pdfium_path = self.ensure_pdfium().await?;
        let input = input.to_path_buf();
        let folder = out_folder.to_path_buf();
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "pdf".to_string());
        tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
            use pdfium_render::prelude::*;
            std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            let bindings = Pdfium::bind_to_library(&pdfium_path).map_err(|e| e.to_string())?;
            let pdfium = Pdfium::new(bindings);
            let src = pdfium
                .load_pdf_from_file(&input, None)
                .map_err(|e| e.to_string())?;
            let total = src.pages().len();
            if total == 0 {
                return Err("o PDF não tem páginas".to_string());
            }
            for i in 0..total {
                let mut dest = pdfium.create_new_pdf().map_err(|e| e.to_string())?;
                dest.pages_mut()
                    .copy_pages_from_document(&src, &format!("{}", i + 1), 0)
                    .map_err(|e| e.to_string())?;
                let path = folder.join(format!("{}_pagina_{}.pdf", stem, i + 1));
                dest.save_to_file(&path).map_err(|e| e.to_string())?;
            }
            Ok(folder)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.into())
    }

    pub async fn rotate_pdf(
        &self,
        input: &Path,
        out: &Path,
        degrees: i32,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let pdfium_path = self.ensure_pdfium().await?;
        let input = input.to_path_buf();
        let out2 = out.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
            use pdfium_render::prelude::*;
            let bindings = Pdfium::bind_to_library(&pdfium_path).map_err(|e| e.to_string())?;
            let pdfium = Pdfium::new(bindings);
            let doc = pdfium
                .load_pdf_from_file(&input, None)
                .map_err(|e| e.to_string())?;
            let rot = match ((degrees % 360) + 360) % 360 {
                90 => PdfPageRenderRotation::Degrees90,
                180 => PdfPageRenderRotation::Degrees180,
                270 => PdfPageRenderRotation::Degrees270,
                _ => PdfPageRenderRotation::None,
            };
            for mut page in doc.pages().iter() {
                page.set_rotation(rot);
            }
            doc.save_to_file(&out2).map_err(|e| e.to_string())?;
            Ok(out2)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.into())
    }

    pub async fn reorder_pdf(
        &self,
        input: &Path,
        out: &Path,
        order: String,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let pdfium_path = self.ensure_pdfium().await?;
        let input = input.to_path_buf();
        let out2 = out.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
            use pdfium_render::prelude::*;
            let order = order.trim();
            if order.is_empty() {
                return Err("informe a nova ordem das páginas (ex.: 3,1,2)".to_string());
            }
            let bindings = Pdfium::bind_to_library(&pdfium_path).map_err(|e| e.to_string())?;
            let pdfium = Pdfium::new(bindings);
            let src = pdfium.load_pdf_from_file(&input, None).map_err(|e| e.to_string())?;
            let mut dest = pdfium.create_new_pdf().map_err(|e| e.to_string())?;
            dest.pages_mut()
                .copy_pages_from_document(&src, order, 0)
                .map_err(|e| e.to_string())?;
            dest.save_to_file(&out2).map_err(|e| e.to_string())?;
            Ok(out2)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.into())
    }

    pub async fn compress_pdf(
        &self,
        input: &Path,
        out: &Path,
        dpi: f32,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let pdfium_path = self.ensure_pdfium().await?;
        let input = input.to_path_buf();
        let out2 = out.to_path_buf();
        let out_ret = out2.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            use pdfium_render::prelude::*;
            let bindings = Pdfium::bind_to_library(&pdfium_path).map_err(|e| e.to_string())?;
            let pdfium = Pdfium::new(bindings);
            let doc = pdfium.load_pdf_from_file(&input, None).map_err(|e| e.to_string())?;
            let total = doc.pages().len();
            if total == 0 {
                return Err("o PDF não tem páginas".to_string());
            }
            let config = PdfRenderConfig::new().scale_page_by_factor((dpi / 72.0) as f32);

            let mut pdoc: Option<printpdf::PdfDocumentReference> = None;
            for (i, page) in doc.pages().iter().enumerate() {
                let bitmap = page.render_with_config(&config).map_err(|e| e.to_string())?;
                let rgb = bitmap.as_image().to_rgb8();
                let (w, h) = rgb.dimensions();
                let xobj = ImageXObject {
                    width: Px(w as usize),
                    height: Px(h as usize),
                    color_space: ColorSpace::Rgb,
                    bits_per_component: ColorBits::Bit8,
                    interpolate: false,
                    image_data: rgb.into_raw(),
                    image_filter: None,
                    smask: None,
                    clipping_bbox: None,
                };
                let wmm = Mm(w as f32 / dpi * 25.4);
                let hmm = Mm(h as f32 / dpi * 25.4);
                if pdoc.is_none() {
                    let (d, pg, layer) =
                        printpdf::PdfDocument::new("Converter", wmm, hmm, "1");
                    let lr = d.get_page(pg).get_layer(layer);
                    printpdf::Image::from(xobj).add_to_layer(
                        lr,
                        ImageTransform { dpi: Some(dpi), ..Default::default() },
                    );
                    pdoc = Some(d);
                } else {
                    let d = pdoc.as_ref().unwrap();
                    let (pg, layer) = d.add_page(wmm, hmm, format!("{}", i + 1));
                    let lr = d.get_page(pg).get_layer(layer);
                    printpdf::Image::from(xobj).add_to_layer(
                        lr,
                        ImageTransform { dpi: Some(dpi), ..Default::default() },
                    );
                }
            }
            let d = pdoc.ok_or("falha ao gerar o PDF")?;
            let file = std::fs::File::create(&out2).map_err(|e| e.to_string())?;
            let mut writer = std::io::BufWriter::new(file);
            d.save(&mut writer).map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())??;
        Ok(out_ret)
    }

    pub(super) async fn image_to_pdf(
        &self,
        input: &Path,
        out: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let input = input.to_path_buf();
        let out_path = out.to_path_buf();
        let out_ret = out_path.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let dynamic = image::open(&input).map_err(|e| e.to_string())?;
            let rgb = dynamic.to_rgb8();
            let (width, height) = rgb.dimensions();

            let xobject = ImageXObject {
                width: Px(width as usize),
                height: Px(height as usize),
                color_space: ColorSpace::Rgb,
                bits_per_component: ColorBits::Bit8,
                interpolate: false,
                image_data: rgb.into_raw(),
                image_filter: None,
                smask: None,
                clipping_bbox: None,
            };

            let dpi = 96.0;
            let width_mm = Mm(width as f32 / dpi * 25.4);
            let height_mm = Mm(height as f32 / dpi * 25.4);

            let (doc, page, layer) =
                PdfDocument::new("Converter", width_mm, height_mm, "Imagem");
            let layer_ref = doc.get_page(page).get_layer(layer);
            Image::from(xobject).add_to_layer(
                layer_ref,
                ImageTransform {
                    dpi: Some(dpi),
                    ..Default::default()
                },
            );

            let file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
            let mut writer = std::io::BufWriter::new(file);
            doc.save(&mut writer).map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(out_ret)
    }

    pub async fn images_to_pdf_multi(
        &self,
        inputs: Vec<PathBuf>,
        out: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let out_path = out.to_path_buf();
        let out_ret = out_path.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            if inputs.is_empty() {
                return Err("nenhuma imagem selecionada".to_string());
            }
            let dpi = 96.0;
            let make_xobject = |path: &Path| -> Result<(ImageXObject, u32, u32), String> {
                let dynamic = image::open(path).map_err(|e| e.to_string())?;
                let rgb = dynamic.to_rgb8();
                let (w, h) = rgb.dimensions();
                Ok((
                    ImageXObject {
                        width: Px(w as usize),
                        height: Px(h as usize),
                        color_space: ColorSpace::Rgb,
                        bits_per_component: ColorBits::Bit8,
                        interpolate: false,
                        image_data: rgb.into_raw(),
                        image_filter: None,
                        smask: None,
                        clipping_bbox: None,
                    },
                    w,
                    h,
                ))
            };

            let (first_xobj, fw, fh) = make_xobject(&inputs[0])?;
            let (doc, page, layer) = PdfDocument::new(
                "Converter",
                Mm(fw as f32 / dpi * 25.4),
                Mm(fh as f32 / dpi * 25.4),
                "Imagem 1",
            );
            let layer_ref = doc.get_page(page).get_layer(layer);
            Image::from(first_xobj).add_to_layer(
                layer_ref,
                ImageTransform {
                    dpi: Some(dpi),
                    ..Default::default()
                },
            );

            for (i, path) in inputs.iter().enumerate().skip(1) {
                let (xobj, w, h) = make_xobject(path)?;
                let (page, layer) = doc.add_page(
                    Mm(w as f32 / dpi * 25.4),
                    Mm(h as f32 / dpi * 25.4),
                    format!("Imagem {}", i + 1),
                );
                let layer_ref = doc.get_page(page).get_layer(layer);
                Image::from(xobj).add_to_layer(
                    layer_ref,
                    ImageTransform {
                        dpi: Some(dpi),
                        ..Default::default()
                    },
                );
            }

            let file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
            let mut writer = std::io::BufWriter::new(file);
            doc.save(&mut writer).map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(out_ret)
    }

    pub(super) async fn pdf_to_text(
        &self,
        input: &Path,
        out: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let input = input.to_path_buf();
        let out_path = out.to_path_buf();
        let out_ret = out_path.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let doc = printpdf::lopdf::Document::load(&input).map_err(|e| e.to_string())?;
            let pages = doc.get_pages();
            let page_numbers: Vec<u32> = pages.keys().copied().collect();
            let text = doc
                .extract_text(&page_numbers)
                .map_err(|e| e.to_string())?;
            if text.trim().is_empty() {
                return Err(
                    "Nenhum texto encontrado (o PDF pode ser apenas imagens escaneadas)."
                        .to_string(),
                );
            }
            std::fs::write(&out_path, text).map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(out_ret)
    }
}

pub(super) fn render_text_pdf(text: &str, out: &Path, title: &str) -> Result<(), String> {
    const PAGE_W: f32 = 210.0;
    const PAGE_H: f32 = 297.0;
    const MARGIN: f32 = 20.0;
    const FONT_SIZE: f32 = 11.0;
    const LINE_H: f32 = 5.0;
    const MAX_CHARS: usize = 92;

    let (doc, page1, layer1) =
        PdfDocument::new(title, Mm(PAGE_W), Mm(PAGE_H), "Texto");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let mut layer = doc.get_page(page1).get_layer(layer1);
    let mut y = PAGE_H - MARGIN;

    let emit = |layer: &mut printpdf::PdfLayerReference,
                y: &mut f32,
                doc: &printpdf::PdfDocumentReference,
                line: &str| {
        if *y < MARGIN {
            let (p, l) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Texto");
            *layer = doc.get_page(p).get_layer(l);
            *y = PAGE_H - MARGIN;
        }
        layer.use_text(line, FONT_SIZE, Mm(MARGIN), Mm(*y), &font);
        *y -= LINE_H;
    };

    for raw_line in text.replace('\t', "    ").lines() {
        let line = raw_line.trim_end();
        if line.is_empty() {
            y -= LINE_H;
            if y < MARGIN {
                let (p, l) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Texto");
                layer = doc.get_page(p).get_layer(l);
                y = PAGE_H - MARGIN;
            }
            continue;
        }
        for wrapped in wrap_line(line, MAX_CHARS) {
            emit(&mut layer, &mut y, &doc, &wrapped);
        }
    }

    let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
    let mut writer = std::io::BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| e.to_string())?;
    Ok(())
}

fn wrap_line(line: &str, max: usize) -> Vec<String> {
    if line.chars().count() <= max {
        return vec![line.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for word in line.split(' ') {
        if word.chars().count() > max {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            let mut chunk = String::new();
            for c in word.chars() {
                chunk.push(c);
                if chunk.chars().count() >= max {
                    out.push(std::mem::take(&mut chunk));
                }
            }
            if !chunk.is_empty() {
                current = chunk;
            }
            continue;
        }
        let extra = if current.is_empty() { 0 } else { 1 };
        if current.chars().count() + extra + word.chars().count() > max {
            out.push(std::mem::take(&mut current));
            current.push_str(word);
        } else {
            if extra == 1 {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}
