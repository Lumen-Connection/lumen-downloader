use std::path::PathBuf;

/// Diretório de dados do app em AppData (config, banco, libs, thumbs, log).
///
/// Migra automaticamente a pasta antiga "LumenDownloader" para "LumenStream"
/// na primeira chamada (rebrand). É idempotente e best-effort: se o rename
/// falhar, cai de volta no nome antigo — nunca perde dados nem começa do zero.
pub fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let new = base.join("LumenStream");
    let old = base.join("LumenDownloader");
    if new.exists() {
        new
    } else if old.exists() {
        match std::fs::rename(&old, &new) {
            Ok(_) => new,
            Err(_) => old,
        }
    } else {
        new
    }
}
