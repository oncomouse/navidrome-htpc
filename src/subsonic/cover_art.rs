//! Cover-art disk + memory cache.
//!
//! [`CoverArtCache`] stores decoded cover-art textures in an in-memory
//! `HashMap<String, TextureHandle>` keyed by cover-id, and keeps the raw
//! bytes on disk under `~/.cache/navidrome-htpc/covers/` so subsequent
//! application starts don't re-download artwork that's already been seen.
//!
//! `fetch_blocking` performs a synchronous fetch (disk hit first, then
//! HTTP GET). It is intentionally blocking — the brief calls for the v1
//! blocking version. A future iteration should move this onto the async
//! Subsonic client thread so the UI thread never blocks on network I/O.

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;

/// Cover-art cache: in-memory textures + on-disk JPEG bytes.
///
/// One instance lives in `NavidromeApp` for the lifetime of the app.
/// The disk cache is shared across runs; the memory cache is rebuilt
/// lazily as covers are requested.
pub struct CoverArtCache {
    /// In-memory decoded textures, keyed by cover-id (NOT by id+size —
    /// the size is fixed at construction time per app run via
    /// `AppState.config.cache.cover_art_size`, so we don't need the
    /// size in the key).
    memory: HashMap<String, egui::TextureHandle>,
    /// Disk cache root, e.g. `~/.cache/navidrome-htpc/covers/`.
    cache_dir: PathBuf,
}

impl CoverArtCache {
    /// Create a new cache rooted at `cache_dir`. The directory (and any
    /// missing parents) is created best-effort; failure is silent because
    /// a missing cache dir simply means we'll fall back to the network path
    /// and skip the disk-write step.
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).ok();
        Self {
            memory: HashMap::new(),
            cache_dir,
        }
    }

    /// Look up a cover-id in the in-memory texture cache. Returns `None`
    /// if the cover hasn't been fetched yet this session — caller should
    /// call `fetch_blocking` (or schedule an async fetch) to populate it.
    pub fn get(&self, id: &str) -> Option<&egui::TextureHandle> {
        self.memory.get(id)
    }

    /// Return a reference to all in-memory textures. Used by the UI loop
    /// to sync cached textures into `AppState::cover_textures` before
    /// rendering.
    pub fn all_textures(&self) -> &HashMap<String, egui::TextureHandle> {
        &self.memory
    }

    /// Insert a pre-built texture (e.g. constructed from bytes already
    /// in hand) into the memory cache. Used by tests and by future async
    /// fetch paths that don't go through `fetch_blocking`.
    pub fn insert(&mut self, id: String, texture: egui::TextureHandle) {
        self.memory.insert(id, texture);
    }

    /// Synchronously fetch, decode, and upload a cover-art texture.
    ///
    /// Flow:
    /// 1. If the in-memory cache has it, return immediately (caller should
    ///    have used `get` first, but this is a safety net).
    /// 2. If the on-disk cache file exists, read it.
    /// 3. Otherwise HTTP GET the URL, write the bytes to disk.
    /// 4. Decode the image, convert to RGBA8, upload as an egui texture,
    ///    insert into the memory cache.
    ///
    /// `size` is the requested pixel size; it's baked into the disk
    /// filename so different sizes don't collide on disk. The memory
    /// cache is keyed by `id` alone (see `memory` doc) — callers should
    /// use a single app-wide size per run.
    pub fn fetch_blocking(
        &mut self,
        ctx: &egui::Context,
        id: &str,
        url: &str,
        size: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fast path: already in memory this session.
        if self.memory.contains_key(id) {
            return Ok(());
        }

        let cache_path = self.cache_dir.join(format!("{id}_{size}.jpg"));
        let bytes = if cache_path.exists() {
            std::fs::read(&cache_path)?
        } else {
            let resp = reqwest::blocking::get(url)?;
            if !resp.status().is_success() {
                return Err(format!("cover-art fetch failed: HTTP {}", resp.status()).into());
            }
            let bytes = resp.bytes()?.to_vec();
            // Best-effort disk write; ignore failures (read-only cache dir,
            // full disk, etc.) — we still have the bytes in memory.
            let _ = std::fs::write(&cache_path, &bytes);
            bytes
        };

        // Decode and upload. `image::load_from_memory` infers format from
        // the bytes, so PNG/GIF/WebP also work despite the .jpg filename.
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        let (w, h) = image.dimensions();
        let color_image =
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &image);
        let texture = ctx.load_texture(id, color_image, egui::TextureOptions::LINEAR);
        self.memory.insert(id.to_string(), texture);
        Ok(())
    }

    /// Drop all in-memory textures. Useful if the user changes the cover
    /// art size setting and we want to force re-decode at the new size.
    pub fn clear_memory(&mut self) {
        self.memory.clear();
    }
}
