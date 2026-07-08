use std::path::PathBuf;

/// Jogos suportados na sincronização de músicas personalizadas.
/// Cada jogo lê áudio de uma pasta própria; por ora só o GTA V.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameTarget {
    GtaV,
}

/// Extensões que o rádio "User Music" do GTA V reconhece diretamente.
/// Qualquer outra é convertida para MP3 antes de copiar.
pub const GTAV_SUPPORTED: &[&str] = &["mp3", "m4a", "aac", "wma"];

/// Pasta onde o GTA V lê as músicas personalizadas:
/// `Documentos\Rockstar Games\GTA V\User Music`.
pub fn gtav_user_music_dir() -> Option<PathBuf> {
    dirs::document_dir().map(|d| {
        d.join("Rockstar Games")
            .join("GTA V")
            .join("User Music")
    })
}

/// Verdadeiro se o arquivo já está num formato que o GTA V toca sem conversão.
pub fn is_gtav_supported(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .map(|e| GTAV_SUPPORTED.contains(&e.as_str()))
        .unwrap_or(false)
}
