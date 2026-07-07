use std::path::PathBuf;

use crate::config::settings::ConvertEngine;

use super::models::{categorize, is_audio_format, AudioMeta, FileCategory};
use super::DownloadEngine;

impl DownloadEngine {
    pub async fn convert_file(
        &self,
        input: &str,
        output_path: &str,
        format: &str,
        preset: &str,
        engine: ConvertEngine,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let input_path = PathBuf::from(input);
        let mut out = PathBuf::from(output_path);
        out.set_extension(format);

        if out == input_path {
            let stem = out
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "convertido".to_string());
            out.set_file_name(format!("{}_convertido.{}", stem, format));
        }

        match categorize(&input_path) {
            FileCategory::Document => {
                if format == "txt" {
                    return self.pdf_to_text(&input_path, &out).await;
                }
                return self.pdf_to_images(&input_path, &out, format).await;
            }
            FileCategory::Office => {
                return self.office_convert(&input_path, &out, format, engine).await;
            }
            _ => {}
        }

        if format == "pdf" {
            return self.image_to_pdf(&input_path, &out).await;
        }

        if is_audio_format(format) {
            self.transcode_audio(&input_path, &out, format, &AudioMeta::default())
                .await?;
        } else {
            self.transcode_media(&input_path, &out, preset).await?;
        }
        Ok(out)
    }
}
