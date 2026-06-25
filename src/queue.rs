//! Fila de downloads com execução de múltiplos itens em paralelo.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::task::JoinHandle;

use crate::app::MediaType;
use crate::db::database::Database;
use crate::download::engine::DownloadEngine;

#[derive(Clone, PartialEq)]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed(String),
    Failed(String),
    Cancelled,
}

pub struct QueueJob {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub media_type: MediaType,
    pub format: String,
    pub quality: String,
    pub folder: PathBuf,
    pub status: JobStatus,
    pub progress: Option<f32>,
}

type Jobs = Arc<Mutex<Vec<QueueJob>>>;

pub struct Queue {
    pub jobs: Jobs,
    pub next_id: Arc<AtomicU64>,
    pub max_concurrent: usize,
    handles: HashMap<u64, JoinHandle<()>>,
}

impl Queue {
    pub fn new() -> Self {
        Queue {
            jobs: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            max_concurrent: 3,
            handles: HashMap::new(),
        }
    }

    pub fn add(
        &self,
        url: String,
        title: String,
        media_type: MediaType,
        format: String,
        quality: String,
        folder: PathBuf,
    ) {
        push_job(
            &self.jobs, &self.next_id, url, title, media_type, format, quality, folder,
        );
    }

    pub fn move_up(&self, id: u64) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(i) = jobs.iter().position(|j| j.id == id) {
            if i > 0 {
                jobs.swap(i, i - 1);
            }
        }
    }

    pub fn move_down(&self, id: u64) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(i) = jobs.iter().position(|j| j.id == id) {
            if i + 1 < jobs.len() {
                jobs.swap(i, i + 1);
            }
        }
    }

    pub fn has_active(&self) -> bool {
        self.jobs
            .lock()
            .unwrap()
            .iter()
            .any(|j| matches!(j.status, JobStatus::Queued | JobStatus::Running))
    }

    /// Pausa um item em andamento (mantém os arquivos parciais para retomar).
    pub fn pause(&mut self, id: u64) {
        if let Some(handle) = self.handles.remove(&id) {
            handle.abort();
        }
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
            if matches!(job.status, JobStatus::Running | JobStatus::Queued) {
                job.status = JobStatus::Paused;
                job.progress = None;
            }
        }
    }

    /// Retoma um item pausado (volta para a fila; o yt-dlp continua o parcial).
    pub fn resume(&mut self, id: u64) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
            if job.status == JobStatus::Paused {
                job.status = JobStatus::Queued;
            }
        }
    }

    pub fn cancel(&mut self, id: u64) {
        if let Some(handle) = self.handles.remove(&id) {
            handle.abort();
        }
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
            if matches!(job.status, JobStatus::Queued | JobStatus::Running) {
                job.status = JobStatus::Cancelled;
            }
        }
    }

    /// Remove itens já finalizados (concluídos/falhos/cancelados).
    pub fn clear_finished(&mut self) {
        self.jobs
            .lock()
            .unwrap()
            .retain(|j| matches!(j.status, JobStatus::Queued | JobStatus::Running));
        self.handles.retain(|_, h| !h.is_finished());
    }

    /// Inicia itens da fila respeitando o limite de execuções simultâneas.
    /// Deve ser chamado a cada frame enquanto houver itens ativos.
    pub fn pump(
        &mut self,
        engine: Arc<DownloadEngine>,
        db_path: PathBuf,
        subtitle_langs: Option<String>,
        notify: bool,
        rate_limit: Option<String>,
        concurrent_fragments: u32,
        organize_by: String,
        cloud_folder: Option<String>,
    ) {
        let running = self
            .jobs
            .lock()
            .unwrap()
            .iter()
            .filter(|j| j.status == JobStatus::Running)
            .count();
        let mut slots = self.max_concurrent.saturating_sub(running);
        if slots == 0 {
            return;
        }

        let mut to_start = Vec::new();
        {
            let mut jobs = self.jobs.lock().unwrap();
            for job in jobs.iter_mut() {
                if slots == 0 {
                    break;
                }
                if job.status == JobStatus::Queued {
                    job.status = JobStatus::Running;
                    to_start.push(job.id);
                    slots -= 1;
                }
            }
        }

        for id in to_start {
            let snapshot = {
                let jobs = self.jobs.lock().unwrap();
                jobs.iter().find(|j| j.id == id).map(|j| {
                    (
                        j.url.clone(),
                        j.media_type,
                        j.format.clone(),
                        j.quality.clone(),
                        j.folder.clone(),
                    )
                })
            };
            let Some((url, media_type, format, quality, folder)) = snapshot else {
                continue;
            };

            let jobs = self.jobs.clone();
            let engine = engine.clone();
            let db_path = db_path.clone();
            let subtitle_langs = subtitle_langs.clone();
            let rate_limit = rate_limit.clone();
            let organize_by = organize_by.clone();
            let cloud_folder = cloud_folder.clone();

            let handle = tokio::spawn(async move {
                let title = match engine.fetch_info(&url).await {
                    Ok(t) => t,
                    Err(e) => {
                        set_status(
                            &jobs,
                            id,
                            JobStatus::Failed(format!("Falha ao obter info: {}", e)),
                        );
                        return;
                    }
                };
                set_title(&jobs, id, title.clone());

                let is_music = media_type == MediaType::Music;
                // Organização automática (tipo/data; canal não disponível na fila).
                let mut folder = folder;
                let media_str = if is_music { "music" } else { "video" };
                if let Some(sub) =
                    crate::download::engine::organize_subfolder(&organize_by, media_str, "")
                {
                    folder = folder.join(sub);
                    let _ = std::fs::create_dir_all(&folder);
                }
                let safe = crate::download::engine::sanitize_filename(&title);
                let out = folder.join(format!("{}.{}", safe, format));
                let out_str = out.to_string_lossy().to_string();

                let jobs_cb = jobs.clone();
                let on_progress = move |p: f64| set_progress(&jobs_cb, id, p as f32);

                let subs = if is_music { None } else { subtitle_langs };
                let opts = crate::download::engine::DownloadOptions {
                    is_audio: is_music,
                    format: format.clone(),
                    quality: quality.clone(),
                    max_height: None,
                    subtitle_langs: subs,
                    clip: None,
                    rate_limit,
                    concurrent_fragments,
                    live_from_start: false,
                };
                match engine
                    .fetch_and_download(&url, &out_str, opts, on_progress)
                    .await
                {
                    Ok(p) => {
                        if let Some(cloud) = &cloud_folder {
                            if let Some(name) = p.file_name() {
                                let dest = std::path::Path::new(cloud).join(name);
                                let _ = std::fs::create_dir_all(cloud);
                                let _ = std::fs::copy(&p, &dest);
                            }
                        }
                        let db = Database::open(&db_path);
                        let file_size = std::fs::metadata(&p).ok().map(|m| m.len() as i64);
                        db.add_history(
                            &url,
                            &title,
                            if is_music { "music" } else { "video" },
                            &format,
                            &quality,
                            &folder.to_string_lossy(),
                            &p.to_string_lossy(),
                            file_size,
                        );
                        set_status(&jobs, id, JobStatus::Completed(p.to_string_lossy().to_string()));
                        if notify {
                            crate::notify::send("Download concluído", &title);
                        }
                    }
                    Err(e) => set_status(&jobs, id, JobStatus::Failed(e.to_string())),
                }
            });
            self.handles.insert(id, handle);
        }
    }
}

/// Adiciona um item à fila. Função livre para permitir adicionar a partir de
/// tarefas em segundo plano (ex.: expansão de playlist).
pub fn push_job(
    jobs: &Jobs,
    next_id: &Arc<AtomicU64>,
    url: String,
    title: String,
    media_type: MediaType,
    format: String,
    quality: String,
    folder: PathBuf,
) {
    let id = next_id.fetch_add(1, Ordering::SeqCst);
    jobs.lock().unwrap().push(QueueJob {
        id,
        url,
        title,
        media_type,
        format,
        quality,
        folder,
        status: JobStatus::Queued,
        progress: None,
    });
}

fn set_status(jobs: &Jobs, id: u64, status: JobStatus) {
    if let Some(job) = jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
        job.status = status;
    }
}
fn set_title(jobs: &Jobs, id: u64, title: String) {
    if let Some(job) = jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
        job.title = title;
    }
}
fn set_progress(jobs: &Jobs, id: u64, p: f32) {
    if let Some(job) = jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
        job.progress = Some(p.clamp(0.0, 1.0));
    }
}

/// Extrai o ID de playlist (`list=`) de uma URL, se houver.
pub fn playlist_id_from_url(url: &str) -> Option<String> {
    url.split(['?', '&'])
        .find_map(|kv| kv.strip_prefix("list="))
        .map(|s| s.to_string())
}

pub fn is_playlist(url: &str) -> bool {
    playlist_id_from_url(url).is_some()
}
