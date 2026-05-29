pub mod connection;

use ::std::path::{Path, PathBuf};
use config::def::widgets::wrapbox::dock::DockConfig;
use connection::{Connection, Event};
use image::imageops::FilterType;
use niri_ipc::{Output, Window, Workspace};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};
use std::collections::{BTreeMap, HashMap};
use std::sync::{mpsc, Arc, RwLock};

use crate::runtime::get_backend_runtime_handle;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct IconKey {
    pub app_id: String,
    pub size: u32,
    pub theme: Option<String>,
    pub fallback: Option<String>,
}

#[derive(Clone, Debug)]
pub enum IconStatus {
    Loading,
    Ready {
        pixels: Arc<[u8]>,
        width: i32,
        height: i32,
        fallback: bool,
    },
    NotFound,
}

#[derive(Default, Debug)]
pub struct NiriDockState {
    pub windows: BTreeMap<u64, Window>,
    pub workspaces: BTreeMap<u64, Workspace>,

    // TODO golbal dock with all outputs odered by logical position
    pub outputs: BTreeMap<u64, Output>,
}

/// Niri Manager:
/// - signals the dock is enabled
/// - carries the redraw sender signal
/// - process icons and populate tha backend icon cache.
/// Bit much?! heh
#[derive(Debug)]
pub struct NiriManager {
    pub state: Arc<RwLock<NiriDockState>>,
    pub icon_cache: Arc<RwLock<HashMap<IconKey, IconStatus>>>,
    redraw_tx: mpsc::Sender<()>,
    icon_size: u32,
    icon_theme: Option<String>,
    icon_fallback: Option<String>,
}

impl NiriManager {
    pub fn new(redraw_tx: mpsc::Sender<()>, config: &DockConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(NiriDockState::default())),
            icon_cache: Arc::new(RwLock::new(HashMap::new())),
            redraw_tx,
            icon_size: config.window_button.icon_size as u32,
            icon_theme: config.window_button.icon_theme.clone(),
            icon_fallback: config.window_button.icon_fallback.clone(),
        }
    }

    fn trigger_redraw(&self) {
        let _ = self.redraw_tx.send(());
    }

    pub fn process_event(&self, e: &Event) {
        let mut state_lock = self.state.write().unwrap();

        match e {
            Event::WorkspaceActivated { id, focused } => {
                if let Some(output) = state_lock.workspaces.get(&id).map(|w| w.output.clone()) {
                    for (ws_id, ws) in state_lock.workspaces.iter_mut() {
                        if ws.output == output {
                            let should_be_active = ws_id == id;
                            let should_be_focused = should_be_active && *focused;

                            if ws.is_active != should_be_active
                                || ws.is_focused != should_be_focused
                            {
                                ws.is_active = should_be_active;
                                ws.is_focused = should_be_focused;
                            }
                        }
                    }
                }
            }
            Event::WorkspacesChanged { workspaces } => {
                state_lock.workspaces.clear();
                state_lock
                    .workspaces
                    .extend(workspaces.into_iter().map(|w| (w.id, w.clone())));
            }
            Event::WindowsChanged { windows } => {
                state_lock.windows.clear();
                state_lock
                    .windows
                    .extend(windows.into_iter().map(|w| (w.id, w.clone())));
            }
            Event::WindowLayoutsChanged { changes, .. } => {
                for (id, new_layout) in changes {
                    if let Some(win) = state_lock.windows.get_mut(&id) {
                        if win.layout != *new_layout {
                            win.layout = new_layout.clone();
                        }
                    }
                }
            }
            Event::WindowOpenedOrChanged { window } => {
                if window.is_focused {
                    state_lock
                        .windows
                        .values_mut()
                        .for_each(|w| w.is_focused = false);
                }
                state_lock.windows.insert(window.id, window.clone());
            }
            Event::WindowClosed { id } => if state_lock.windows.remove(&id).is_some() {},
            Event::WindowFocusChanged { id } => {
                if let Some(focused_id) = id {
                    for (win_id, win) in state_lock.windows.iter_mut() {
                        let should_be_focused = win_id == focused_id;
                        if win.is_focused != should_be_focused {
                            win.is_focused = should_be_focused;
                        }
                    }
                }
            }
            Event::WindowUrgencyChanged { id, is_urgent } => {
                if let Some(win) = state_lock.windows.get_mut(&id) {
                    if win.is_urgent != *is_urgent {
                        win.is_urgent = *is_urgent;
                    }
                }
            }
        }
        drop(state_lock);

        //redrawing all events! If in the futura some event won't need redraw, then the redraw must
        //go from here to inside each match case where it's needed
        self.trigger_redraw();
    }

    /// Gets newly opened window icon
    pub fn process_window_opened(&self, window: &Window) {
        let app_id = window.app_id.clone().unwrap_or_default();

        let icon_key = IconKey {
            app_id,
            theme: self.icon_theme.clone(),
            size: self.icon_size,
            fallback: self.icon_fallback.clone(),
        };

        let mut cache = self.icon_cache.write().unwrap();

        if !cache.contains_key(&icon_key) {
            cache.insert(icon_key.clone(), IconStatus::Loading);

            let cache_clone = Arc::clone(&self.icon_cache);
            let redraw_tx_clone = self.redraw_tx.clone();

            get_backend_runtime_handle().spawn(async move {
                Self::load_and_rasterize_icon_async(cache_clone, icon_key, redraw_tx_clone).await;
            });
        }
    }

    /// Load cached icons or fallback
    async fn load_and_rasterize_icon_async(
        cache_arc: Arc<RwLock<HashMap<IconKey, IconStatus>>>,
        icon: IconKey,
        redraw_tx: mpsc::Sender<()>,
    ) {
        let trigger_redraw = || {
            let _ = redraw_tx.send(());
        };

        // 1st: look for the main icon
        if let Some((pixels, width, height)) = Self::fetch_icon_pixels(&icon).await {
            Self::update_cache_status(
                &cache_arc,
                icon,
                IconStatus::Ready {
                    pixels: pixels.into(),
                    width,
                    height,
                    fallback: false,
                },
            );

            return trigger_redraw();
        }

        // 2nd: valodate fallback icon
        let fallback_str = match &icon.fallback {
            Some(path) if Path::new(path).is_file() => path.clone(),
            _ => {
                Self::update_cache_status(&cache_arc, icon, IconStatus::NotFound);
                return trigger_redraw();
            }
        };

        let fb_key = IconKey {
            app_id: fallback_str,
            size: icon.size,
            theme: icon.theme.clone(),
            fallback: icon.fallback.clone(),
        };

        // 3rd: checks if fallback is cached
        {
            let mut cache = cache_arc.write().unwrap();
            match cache.get(&fb_key) {
                Some(IconStatus::Ready {
                    pixels,
                    width,
                    height,
                    ..
                }) => {
                    let status = IconStatus::Ready {
                        pixels: pixels.clone(),
                        width: *width,
                        height: *height,
                        fallback: true,
                    };
                    cache.insert(icon, status);
                    return trigger_redraw();
                }
                Some(IconStatus::Loading) => {
                    cache.insert(icon, IconStatus::Loading);
                    return;
                }
                Some(IconStatus::NotFound) => {
                    cache.insert(icon, IconStatus::NotFound);
                    return trigger_redraw();
                }
                None => {
                    cache.insert(icon.clone(), IconStatus::Loading);
                    cache.insert(fb_key.clone(), IconStatus::Loading);
                }
            }
        }

        // 4th: looks for fallback
        match Self::fetch_icon_pixels(&fb_key).await {
            Some((pixels, width, height)) => {
                let status = IconStatus::Ready {
                    pixels: pixels.into(),
                    width,
                    height,
                    fallback: true,
                };
                let mut cache = cache_arc.write().unwrap();
                cache.insert(fb_key, status.clone());
                cache.insert(icon, status);
            }
            None => {
                Self::update_cache_status(&cache_arc, icon, IconStatus::NotFound);
            }
        }

        trigger_redraw();
    }

    fn get_icon_from_desktop_file(app_id: &str) -> Option<String> {
        let mut search_paths = vec![
            PathBuf::from(format!("/usr/share/applications/{}.desktop", app_id)),
            PathBuf::from(format!("/usr/local/share/applications/{}.desktop", app_id)),
        ];

        if let Ok(home) = std::env::var("HOME") {
            search_paths.push(PathBuf::from(format!(
                "{}/.local/share/applications/{}.desktop",
                home, app_id
            )));
        }

        for path in search_paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Icon=") {
                        return Some(trimmed.trim_start_matches("Icon=").to_string());
                    }
                }
            }
        }

        None
    }

    /// Async helper for disk searches e image processing
    async fn fetch_icon_pixels(icon: &IconKey) -> Option<(Vec<u8>, i32, i32)> {
        let (app_id, theme, size) = (icon.app_id.to_string(), icon.theme.clone(), icon.size);

        tokio::task::spawn_blocking(move || {
            // 1st: Direct file path
            let path = Path::new(&app_id);
            if path.is_file() {
                if let Some(mut pixels) = process_image_file(path, size) {
                    swizzle_rgba_to_bgra(&mut pixels);
                    return Some((pixels, size as i32, size as i32));
                }
                return None;
            }

            // 2nd: Icon search setup
            let (app_id_lower, app_id_short) = generate_search_variants(&app_id);
            let ordered_sizes = generate_ordered_sizes(size);

            let mut names_to_try = Vec::new();

            if let Some(desktop_icon_name) = Self::get_icon_from_desktop_file(&app_id) {
                names_to_try.push(desktop_icon_name);
            }

            for name in [app_id.clone(), app_id_lower, app_id_short] {
                if !names_to_try.contains(&name) {
                    names_to_try.push(name);
                }
            }

            // 3rd: A busca propriamente dita (substituindo a closure e o unwrap perigoso)
            for name in &names_to_try {
                for &size_u16 in &ordered_sizes {
                    let mut builder = freedesktop_icons::lookup(name).with_size(size_u16);

                    // Adiciona o tema APENAS se o usuário configurou um válido
                    if let Some(t) = &theme {
                        if !t.is_empty() {
                            builder = builder.with_theme(t);
                        }
                    }

                    if let Some(path) = builder.find() {
                        if let Some(mut pixels) = process_image_file(&path, size) {
                            swizzle_rgba_to_bgra(&mut pixels);
                            return Some((pixels, size as i32, size as i32));
                        }
                    }
                }
            }

            None
        })
        .await
        .unwrap_or(None)
    }

    /// Async helper to update cache status
    fn update_cache_status(
        cache_arc: &Arc<RwLock<HashMap<IconKey, IconStatus>>>,
        key: IconKey,
        status: IconStatus,
    ) {
        let mut cache = cache_arc.write().unwrap();
        cache.insert(key, status);
    }
}

/// Backend start point
pub fn init_and_sync(
    sync_workspaces: bool,
    sync_windows: bool,
    redraw_tx: Option<mpsc::Sender<()>>,
    config: Option<&DockConfig>,
) -> Option<Arc<NiriManager>> {
    let manager = if let (Some(tx), Some(cfg)) = (redraw_tx.clone(), config) {
        Some(Arc::new(NiriManager::new(tx, cfg)))
    } else {
        None
    };

    let manager_for_frontend = manager.clone();

    get_backend_runtime_handle().spawn(async move {
        let mut connection = match Connection::make_connection().await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to connect to Niri IPC on initialization {e}");
                return;
            }
        };

        if sync_workspaces {
            if let Ok(Ok(niri_ipc::Response::Workspaces(ws))) =
                connection.push_request(niri_ipc::Request::Workspaces).await
            {
                if let Some(mgr) = &manager {
                    let event = Event::WorkspacesChanged { workspaces: ws };
                    mgr.process_event(&event);
                }
            }
        }

        if sync_windows {
            if let Ok(Ok(niri_ipc::Response::Windows(wins))) =
                connection.push_request(niri_ipc::Request::Windows).await
            {
                if let Some(mgr) = &manager {
                    for win in &wins {
                        mgr.process_window_opened(win);
                    }
                    let event = Event::WindowsChanged { windows: wins };
                    mgr.process_event(&event);
                }
            }
        }

        start_central_listener_loop(connection, manager).await;
    });
    manager_for_frontend
}

async fn start_central_listener_loop(connection: Connection, manager: Option<Arc<NiriManager>>) {
    let mut listener = match connection.to_listener().await {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to start Niri EventStream {e}");
            return;
        }
    };

    let mut buf = String::new();

    loop {
        match listener.next_event(&mut buf).await {
            Ok(Some(e)) => match &e {
                // 1. Workspaces Events for workspaces and dock widgets
                Event::WorkspaceActivated { .. } | Event::WorkspacesChanged { .. } => {
                    crate::workspace::niri::process_event(e.clone()).await;
                    if let Some(mgr) = &manager {
                        mgr.process_event(&e);
                    }
                }

                // 2. Windows events, only for dock widget
                Event::WindowOpenedOrChanged { window } => {
                    if let Some(mgr) = &manager {
                        mgr.process_window_opened(window);
                        mgr.process_event(&e);
                    }
                }
                Event::WindowsChanged { windows } => {
                    if let Some(mgr) = &manager {
                        for win in windows {
                            mgr.process_window_opened(win);
                        }
                        mgr.process_event(&e);
                    }
                }
                Event::WindowClosed { .. }
                | Event::WindowLayoutsChanged { .. }
                | Event::WindowUrgencyChanged { .. }
                | Event::WindowFocusChanged { .. } => {
                    if let Some(mgr) = &manager {
                        mgr.process_event(&e);
                    }
                }
            },
            Ok(None) => {}
            Err(err) => {
                log::error!("Error reading Niri EventStream {err}");
                break;
            }
        }
        buf.clear();
    }
}

// ==========================================
// RENDER AND ICON SEARCH UTILS
// ==========================================

fn process_image_file(path: &Path, size: u32) -> Option<Vec<u8>> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match extension {
        "svg" => rasterize_svg(path, size),
        "png" | "jpg" | "jpeg" => decode_and_resize_raster(path, size),
        _ => None,
    }
}

fn rasterize_svg(path: &Path, size: u32) -> Option<Vec<u8>> {
    let svg_data = std::fs::read(path).ok()?;
    let opt = Options::default();
    let tree = Tree::from_data(&svg_data, &opt).ok()?;
    let mut pixmap = Pixmap::new(size, size)?;

    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let transform = Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap.take())
}

fn decode_and_resize_raster(path: &Path, size: u32) -> Option<Vec<u8>> {
    let img = image::open(path).ok()?;
    let resized = img.resize_exact(size, size, FilterType::Lanczos3);
    let mut rgba_img = resized.into_rgba8();

    for pixel in rgba_img.pixels_mut() {
        let alpha = pixel[3] as f32 / 255.0;
        pixel[0] = (pixel[0] as f32 * alpha) as u8;
        pixel[1] = (pixel[1] as f32 * alpha) as u8;
        pixel[2] = (pixel[2] as f32 * alpha) as u8;
    }
    Some(rgba_img.into_raw())
}

#[inline(always)]
fn swizzle_rgba_to_bgra(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let r = chunk[0];
        let b = chunk[2];
        chunk[0] = b;
        chunk[2] = r;
    }
}

fn generate_search_variants(app_id: &str) -> (String, String) {
    let app_id_lower = app_id.to_lowercase();
    let app_id_short = app_id_lower
        .split('.')
        .last()
        .unwrap_or(&app_id_lower)
        .to_string();
    (app_id_lower, app_id_short)
}

fn generate_ordered_sizes(target_size: u32) -> Vec<u16> {
    const STANDARD_SIZES: [u16; 21] = [
        8, 16, 20, 22, 24, 28, 32, 36, 42, 44, 48, 64, 72, 96, 128, 150, 192, 256, 384, 512, 1024,
    ];
    let partition_point = STANDARD_SIZES.partition_point(|&x| x < target_size as u16);
    STANDARD_SIZES[partition_point..]
        .iter()
        .chain(STANDARD_SIZES[..partition_point].iter().rev())
        .copied()
        .collect()
}
