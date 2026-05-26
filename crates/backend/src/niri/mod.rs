pub mod connection;

use ::std::path::Path;
use connection::{Connection, Event};
use image::imageops::FilterType;
use niri_ipc::Window;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};
use std::sync::atomic::{AtomicBool, Ordering};

static LISTENER_STARTED: AtomicBool = AtomicBool::new(false);
pub static REDRAW_SIGNAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn init_and_sync(sync_workspaces: bool, sync_windows: bool) {
    get_backend_runtime_handle().spawn(async move {
        if sync_workspaces {
            if let Ok(mut l) = Connection::make_connection().await {
                if let Ok(Ok(niri_ipc::Response::Workspaces(ws))) =
                    l.push_request(niri_ipc::Request::Workspaces).await
                {
                    crate::dock::niri::process_event(Event::WorkspacesChanged { workspaces: ws })
                        .await;
                }
            }
        }

        if sync_windows {
            if let Ok(mut l) = Connection::make_connection().await {
                if let Ok(Ok(niri_ipc::Response::Windows(wins))) =
                    l.push_request(niri_ipc::Request::Windows).await
                {
                    for win in &wins {
                        process_window_opened(win);
                    }
                    crate::dock::niri::process_event(Event::WindowsChanged { windows: wins }).await;
                }
            }
        }

        if !LISTENER_STARTED.swap(true, Ordering::Relaxed) {
            start_central_listener_loop().await;
        }
    });
}

async fn start_central_listener_loop() {
    let mut l = Connection::make_connection()
        .await
        .expect("Failed to connect to niri socket")
        .to_listener()
        .await
        .expect("Failed to send EventStream request");

    let mut buf = String::new();
    loop {
        match l.next_event(&mut buf).await {
            Ok(Some(e)) => {
                match &e {
                    // Rota 1: Workspaces (Sempre ativo)
                    Event::WorkspaceActivated { .. } | Event::WorkspacesChanged { .. } => {
                        crate::workspace::niri::process_event(e).await;
                    }

                    Event::WindowOpenedOrChanged { window } => {
                        process_window_opened(&window);

                        if crate::dock::DOCK_ENABLED.load(Ordering::Relaxed) {
                            crate::dock::niri::process_event(e).await;
                        }
                    }
                    Event::WindowsChanged { windows } => {
                        if crate::dock::DOCK_ENABLED.load(Ordering::Relaxed) {
                            for win in windows {
                                process_window_opened(win);
                            }
                            crate::dock::niri::process_event(Event::WindowsChanged {
                                windows: windows.clone(),
                            })
                            .await;
                        }
                    }
                    // Rota 2: Janelas (Custo Zero Condicional)
                    Event::WindowClosed { .. }
                    | Event::WindowLayoutsChanged { .. }
                    | Event::WindowUrgencyChanged { .. }
                    | Event::WindowFocusChanged { .. } => {
                        if crate::dock::DOCK_ENABLED.load(Ordering::Relaxed) {
                            crate::dock::niri::process_event(e).await;
                        }
                    }
                }
            }
            Ok(None) => {}
            Err(err) => {
                log::error!("error reading from niri event stream: {err}");
                break;
            }
        }
        buf.clear();
    }
}

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct IconCacheKey {
    pub app_id: String,
    pub size: u32,
    pub theme: Option<String>,
    pub fallback: Option<String>,
}

#[derive(Clone)]
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

use crate::dock::niri::ICON_CONFIG;
use crate::runtime::get_backend_runtime_handle;

// Cache global mapeando app_id para o status do ícone
pub static ICON_CACHE: Lazy<Arc<RwLock<HashMap<IconCacheKey, IconStatus>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

fn process_window_opened(window: &Window) {
    let app_id = window.app_id.clone().unwrap_or_default();

    let (theme, fallback, size) = {
        let cfg = ICON_CONFIG.read().unwrap();
        (cfg.theme.clone(), cfg.fallback.clone(), cfg.size as u32)
    };

    let mut cache = ICON_CACHE.write().unwrap();

    let icon = IconCacheKey {
        app_id: app_id.clone(),
        theme: theme,
        size: size,
        fallback: fallback,
    };

    // Se o ícone não está no cache, inicia o processo de extração
    if !cache.contains_key(&icon) {
        cache.insert(icon.clone(), IconStatus::Loading);

        // Dispara a carga pesada para o pool de threads do Tokio
        get_backend_runtime_handle().spawn(async move {
            load_and_rasterize_icon_async(&icon).await;
        });
    }
}

async fn image_sync(icon: &IconCacheKey) -> Option<(Vec<u8>, i32, i32)> {
    let (app_id_owned, theme, size) = { (icon.app_id.to_string(), icon.theme.clone(), icon.size) };

    tokio::task::spawn_blocking(move || {
        // Rota A: É um caminho absoluto no disco (Usado primariamente pelo fallback)
        let path = Path::new(&app_id_owned);
        if path.is_file() {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let raw_pixels = match extension {
                "svg" => rasterize_svg(path, size),
                "png" | "jpg" | "jpeg" => decode_and_resize_raster(path, size),
                _ => None,
            };

            if let Some(mut pixels) = raw_pixels {
                swizzle_rgba_to_bgra(&mut pixels);
                return Some((pixels, size as i32, size as i32));
            }
            return None;
        }

        // Rota B: É um ID abstrato. Gerar variantes para o linicon.
        // Ex: "org.gnome.Nautilus" vira ["org.gnome.Nautilus", "org.gnome.nautilus", "nautilus"]
        let app_id_lower = app_id_owned.to_lowercase();
        let app_id_short = app_id_lower
            .split('.')
            .last()
            .unwrap_or(&app_id_lower)
            .to_string();

        const STANDARD_SIZES: [u16; 21] = [
            8, 16, 20, 22, 24, 28, 32, 36, 42, 44, 48, 64, 72, 96, 128, 150, 192, 256, 384, 512,
            1024,
        ];

        let partition_point = STANDARD_SIZES.partition_point(|&x| x < size as u16);

        // Array ordered from the next size bigger from de target size until the max
        // and then ordered from the next size smaller to the minimum
        let ordered_sizes: Vec<u16> = STANDARD_SIZES[partition_point..]
            .iter()
            .chain(STANDARD_SIZES[..partition_point].iter().rev())
            .copied()
            .collect();

        let mut names_to_try = vec![app_id_owned.to_string()];
        if !names_to_try.contains(&app_id_lower) {
            names_to_try.push(app_id_lower.clone());
        }

        if !names_to_try.contains(&app_id_short) {
            names_to_try.push(app_id_short);
        }

        for name in names_to_try {
            for size_u16 in ordered_sizes.clone() {
                let mut queries = Vec::new();

                if let Some(ref t) = theme {
                    queries.push(
                        linicon::lookup_icon(&name)
                            .from_theme(t)
                            .with_size(size_u16),
                    );
                }
                queries.push(linicon::lookup_icon(&name).with_size(size_u16));
                queries.push(
                    linicon::lookup_icon(&name)
                        .from_theme("hicolor")
                        .with_size(size_u16),
                );

                for query in queries {
                    for icon in query.into_iter().filter_map(Result::ok) {
                        if !icon.path.exists() {
                            continue;
                        }

                        let extension =
                            icon.path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        let raw_pixels = match extension {
                            "svg" => rasterize_svg(&icon.path, size),
                            "png" | "jpg" | "jpeg" => decode_and_resize_raster(&icon.path, size),
                            _ => continue,
                        };
                        match raw_pixels {
                            Some(mut pixels) => {
                                swizzle_rgba_to_bgra(&mut pixels);
                                return Some((pixels, size as i32, size as i32));
                            }
                            None => {
                                continue; // Tenta o próximo ícone da lista
                            }
                        }
                    }
                }
            }
        }

        None
    })
    .await
    .unwrap_or(None)
}

async fn load_and_rasterize_icon_async(icon: &IconCacheKey) {
    // Extract icon configs
    let (theme, fallback, size) = { (icon.theme.clone(), icon.fallback.clone(), icon.size) };

    // set redraw function used when returning
    let trigger_redraw = || {
        REDRAW_SIGNAL.store(true, std::sync::atomic::Ordering::Relaxed);
    };

    // try to get app icon
    let pixel_data = image_sync(&icon).await;

    let fallback_to_fetch: IconCacheKey;

    {
        // open icon cache
        let mut cache = ICON_CACHE.write().unwrap();

        // Return 1: got icon sucessfully
        if let Some((pixels, width, height)) = pixel_data {
            cache.insert(
                icon.clone(),
                IconStatus::Ready {
                    pixels: pixels.into(),
                    width,
                    height,
                    fallback: false,
                },
            );
            return trigger_redraw();
        }

        // Return 2: check for fallback string if is path to an image
        let fb_str = match &fallback {
            Some(path_str) if Path::new(&path_str).is_file() => path_str,
            _ => {
                cache.insert(icon.clone(), IconStatus::NotFound);
                return trigger_redraw();
            }
        };

        let fb_icon = IconCacheKey {
            app_id: fb_str.clone(),
            size,
            theme,
            fallback,
        };

        // try to get fallback icon
        let fallback_data = match cache.get(&fb_icon) {
            Some(IconStatus::Ready {
                pixels,
                width,
                height,
                ..
            }) => Some((pixels.clone(), *width, *height)),
            _ => None,
        };

        // Return 3: points app icon cache to fallback icon
        if let Some((pixels_clone, w, h, ..)) = fallback_data {
            cache.insert(
                icon.clone(),
                IconStatus::Ready {
                    pixels: pixels_clone,
                    width: w,
                    height: h,
                    fallback: true,
                },
            );
            return trigger_redraw();
        }

        // if no fallback icon, set app cache to Loading
        cache.insert(icon.clone(), IconStatus::Loading);

        fallback_to_fetch = fb_icon;
    }

    let fb_pixel_data = image_sync(&fallback_to_fetch).await;

    // open icon cache
    let mut cache = ICON_CACHE.write().unwrap();

    // Return 4: got icon sucessfully
    match fb_pixel_data {
        Some((pixels, width, height)) => {
            let icon_status = IconStatus::Ready {
                pixels: pixels.into(),
                width,
                height,
                fallback: true,
            };
            cache.insert(fallback_to_fetch.clone(), icon_status.clone());
            cache.insert(icon.clone(), icon_status);
        }
        None => {
            cache.insert(icon.clone(), IconStatus::NotFound);
        }
    }
    trigger_redraw();
}

// ==========================================
// FUNÇÕES AUXILIARES DE PROCESSAMENTO (MANTENHA NO MESMO ARQUIVO)
// ==========================================

fn rasterize_svg(path: &std::path::Path, size: u32) -> Option<Vec<u8>> {
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

fn decode_and_resize_raster(path: &std::path::Path, size: u32) -> Option<Vec<u8>> {
    let img = image::open(path).ok()?;
    let resized = img.resize_exact(size, size, FilterType::Lanczos3);
    let mut rgba_img = resized.into_rgba8();

    // Multiplicação manual do Alpha para matrizes estáticas PNG
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
        chunk[0] = b; // Move Red para a posição do Blue
        chunk[2] = r; // Move Blue para a posição do Red
    }
}
