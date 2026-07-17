use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameTarget {
    GtaV,
}

pub const GTAV_SUPPORTED: &[&str] = &["mp3", "m4a", "aac", "wma"];

/// Nomes das pastas de edição do GTA V no PC, em ordem de preferência:
/// "GTAV Enhanced" (edição de 2025) e "GTA V" (Legacy). As duas usam pastas
/// separadas dentro de Documentos\Rockstar Games — daí o descasamento se
/// mirarmos só uma.
const GTAV_EDITIONS: &[&str] = &["GTAV Enhanced", "GTA V"];

/// Pastas "User Music" das edições do GTA V instaladas. Sincronizar em todas as
/// que existirem cobre tanto a Enhanced quanto a Legacy sem o usuário precisar
/// escolher. Se nenhuma existir ainda, mira na Enhanced (versão atual do PC),
/// que é criada na hora de sincronizar.
pub fn gtav_user_music_dirs() -> Vec<PathBuf> {
    let Some(docs) = dirs::document_dir() else {
        return Vec::new();
    };
    let rockstar = docs.join("Rockstar Games");
    let mut dirs: Vec<PathBuf> = GTAV_EDITIONS
        .iter()
        .map(|e| rockstar.join(e))
        .filter(|d| d.exists())
        .map(|d| d.join("User Music"))
        .collect();
    if dirs.is_empty() {
        dirs.push(rockstar.join("GTAV Enhanced").join("User Music"));
    }
    dirs
}

pub fn is_gtav_supported(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .map(|e| GTAV_SUPPORTED.contains(&e.as_str()))
        .unwrap_or(false)
}
