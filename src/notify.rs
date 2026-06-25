//! Notificação de desktop (best-effort).

/// Mostra uma notificação nativa. Ignora falhas silenciosamente.
pub fn send(summary: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .appname("Lumen Downloader")
        .show();
}
