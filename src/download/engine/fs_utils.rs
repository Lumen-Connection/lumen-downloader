use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) fn binary_path(dir: &PathBuf, name: &str) -> PathBuf {
    if cfg!(windows) {
        dir.join(format!("{}.exe", name))
    } else {
        dir.join(name)
    }
}

pub fn cleanup_partials(folder: &Path, stem: &str) {
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            // "ext" está em minúsculas — o sufixo dos fragmentos ("...part-FragN")
            // precisa ser comparado também em minúsculas.
            let is_temp_ext = matches!(ext.as_str(), "part" | "ytdl" | "temp" | "rawaudio" | "recseg")
                || ext.starts_with("part-frag");
            if name.starts_with(stem) && is_temp_ext {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

pub fn cleanup_temp_dir(folder: &Path) {
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if matches!(ext.as_str(), "part" | "ytdl" | "rawaudio")
                || name.starts_with("temp_audio_")
                || name.starts_with("temp_video_")
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// Total de bytes nos fragmentos ".part-FragN" completos de uma gravação DVR.
pub(super) fn frag_bytes(folder: &Path, stem: &str) -> u64 {
    let mut total = 0u64;
    if let Ok(rd) = std::fs::read_dir(folder) {
        for entry in rd.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(stem)
                    && name.contains(".part-Frag")
                    && !name.ends_with(".part")
                {
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }
    total
}

/// Concatena os fragmentos ".part-FragN" de cada formato (ex.: f136 vídeo, f140
/// áudio) em arquivos contínuos, em ordem numérica. Retorna os arquivos gerados,
/// do maior (vídeo) para o menor (áudio). Método validado: fmp4 do YouTube
/// concatenado assim remuxa num mp4 íntegro.
pub(super) async fn concat_frag_groups(folder: &Path, stem: &str) -> Vec<PathBuf> {
    let folder = folder.to_path_buf();
    let stem = stem.to_string();
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let mut groups: HashMap<String, Vec<(u64, PathBuf)>> = HashMap::new();
        let Ok(rd) = std::fs::read_dir(&folder) else {
            return Vec::new();
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            // Ignora o frag em trânsito (ainda .part no fim), que pode estar truncado.
            if !name.starts_with(&stem) || name.ends_with(".part") {
                continue;
            }
            let Some(pos) = name.find(".part-Frag") else {
                continue;
            };
            let Ok(num) = name[pos + ".part-Frag".len()..].parse::<u64>() else {
                continue;
            };
            groups
                .entry(name[..pos].to_string())
                .or_default()
                .push((num, path));
        }
        let mut outs: Vec<(u64, PathBuf)> = Vec::new();
        for (key, mut frags) in groups {
            frags.sort_by_key(|(n, _)| *n);
            let dest = folder.join(format!("{}.recseg", key));
            let Ok(f) = std::fs::File::create(&dest) else {
                continue;
            };
            let mut w = std::io::BufWriter::new(f);
            let mut total = 0u64;
            for (_, p) in &frags {
                if let Ok(bytes) = std::fs::read(p) {
                    total += bytes.len() as u64;
                    if w.write_all(&bytes).is_err() {
                        break;
                    }
                }
            }
            let _ = w.flush();
            if total > 0 {
                outs.push((total, dest));
            } else {
                let _ = std::fs::remove_file(&dest);
            }
        }
        outs.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
        outs.into_iter().map(|(_, p)| p).collect()
    })
    .await
    .unwrap_or_default()
}

/// Soma o que a gravação já escreveu no disco: `.part` e também os fragmentos
/// `.part-FragN` que o modo DVR (--live-from-start) mantém separados até o fim.
///
/// Nota Windows/NTFS: enquanto o ffmpeg mantém o arquivo aberto, o tamanho na
/// entrada do diretório fica defasado (o Explorer mostra 0 KB!). Por isso, para
/// os `.part` principais o tamanho é lido do handle (File::open + metadata),
/// que reflete o valor real; fragmentos já fechados usam a entrada do diretório.
pub fn part_bytes(folder: &Path, stem: &str) -> u64 {
    let mut total = 0u64;
    if let Ok(rd) = std::fs::read_dir(folder) {
        for entry in rd.flatten() {
            let Some(name) = entry.file_name().to_str().map(|s| s.to_string()) else {
                continue;
            };
            if !name.starts_with(stem) || !name.contains(".part") {
                continue;
            }
            let dir_len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let main_part = name.ends_with(".part") && !name.contains("-Frag");
            total += if main_part || dir_len == 0 {
                std::fs::File::open(entry.path())
                    .and_then(|f| f.metadata())
                    .map(|m| m.len())
                    .unwrap_or(dir_len)
            } else {
                dir_len
            };
        }
    }
    total
}

pub(super) fn find_output(folder: &Path, stem: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(folder).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let matches_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy() == stem)
            .unwrap_or(false);
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if matches_stem && !matches!(ext.as_str(), "srt" | "vtt" | "part" | "ytdl") {
            return Some(path);
        }
    }
    None
}

pub(super) fn find_named(dir: &Path, target: &str, depth: u32) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if target.eq_ignore_ascii_case(name) {
                return Some(path);
            }
        }
    }
    if depth > 0 {
        for sub in subdirs {
            if let Some(found) = find_named(&sub, target, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}
