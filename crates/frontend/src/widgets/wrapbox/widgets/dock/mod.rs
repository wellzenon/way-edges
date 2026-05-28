pub mod draw;
pub mod layout;

use crate::mouse_state::MouseEvent;
use crate::widgets::wrapbox::box_traits::BoxedWidget;
use crate::widgets::wrapbox::BoxTemporaryCtx;
use backend::niri::{IconKey, IconStatus, NiriManager};
use cairo::{Format, ImageSurface};
use config::def::widgets::wrapbox::dock::DockConfig;
use cosmic_text::{FontSystem, SwashCache};
use smithay_client_toolkit::output::OutputData;
use std::{collections::HashMap, process::Command, sync::Arc};
use tokio::sync::mpsc;
use wayland_client::Proxy;

use layout::DockLayout;

#[derive(Debug)]
pub struct DockWidget {
    pub config: DockConfig,
}

impl DockWidget {
    pub fn new(config: DockConfig) -> Self {
        Self { config }
    }
}

#[derive(Debug)]
pub struct DockCtx {
    pub output: String,
    pub widget: DockWidget,
    pub last_layout: DockLayout,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub icon_surface_cache: HashMap<IconKey, ImageSurface>,
    pub glyph_cache: HashMap<cosmic_text::CacheKey, ImageSurface>,
    pub niri_manager: Option<Arc<NiriManager>>,
}

impl DockCtx {
    /// RECONCICLIATION: Tranforms the backend cache (NiriManager.icon_cache)
    /// into Cairo ImageSurface and populates the frontend cache (icon_surface_cache)
    fn reconcile_icons(&mut self, windows: &[niri_ipc::Window]) {
        let Some(manager) = &self.niri_manager else {
            return;
        };
        let backend_cache = manager.icon_cache.read().unwrap();
        let config = &self.widget.config.window_button;

        for win in windows {
            let app_id = win.app_id.clone().unwrap_or_default();
            let key = IconKey {
                app_id,
                theme: config.icon_theme.clone(),
                size: config.icon_size as u32,
                fallback: config.icon_fallback.clone(),
            };

            if self.icon_surface_cache.contains_key(&key) {
                continue;
            }

            if let Some(IconStatus::Ready {
                pixels,
                width,
                height,
                ..
            }) = backend_cache.get(&key)
            {
                if let Ok(mut surface) = ImageSurface::create(Format::ARgb32, *width, *height) {
                    let stride = surface.stride() as usize;
                    if let Ok(mut data) = surface.data() {
                        let w_bytes = (*width * 4) as usize;
                        for (dest_row, src_row) in data
                            .chunks_exact_mut(stride)
                            .zip(pixels.chunks_exact(w_bytes))
                        {
                            dest_row[..w_bytes].copy_from_slice(src_row);
                        }
                    }
                    surface.mark_dirty();
                    self.icon_surface_cache.insert(key, surface);
                }
            }
        }
    }
}

impl BoxedWidget for DockCtx {
    fn content(&mut self) -> ImageSurface {
        // 1. Extract worlspaces and windows state, initially
        // TODO outputs state
        let (workspaces, mut windows) = if let Some(manager) = &self.niri_manager {
            let state = manager.state.write().unwrap();
            // filter workspaces by output
            let ws: std::collections::BTreeMap<_, _> = state
                .workspaces
                .iter()
                .filter(|(_, w)| &w.output.as_deref().unwrap_or_default() == &self.output)
                .map(|(&k, w)| (k, w.clone()))
                .collect();

            // filter windows by workspace
            let wins: Vec<_> = state
                .windows
                .values()
                .filter(|w| w.workspace_id.map_or(false, |id| ws.contains_key(&id)))
                .cloned()
                .collect();

            (ws, wins)
        } else {
            (std::collections::BTreeMap::new(), Vec::new())
        };

        // 2. order by workspace idx (position from top to bottom) then floats and last by window x,y position
        windows.sort_by_key(|w| {
            let ws_idx = workspaces
                .get(&w.workspace_id.unwrap_or_default())
                .map(|ws| ws.idx)
                .unwrap_or_default();

            (ws_idx, w.is_floating, w.layout.pos_in_scrolling_layout)
        });

        self.reconcile_icons(&windows);

        // Builds the layout
        self.last_layout = DockLayout::calculate(
            &windows,
            &workspaces,
            &self.widget.config,
            &mut self.icon_surface_cache,
            &mut self.font_system,
        );

        // 4. Main ImageSurface
        let width = self.last_layout.total_width.max(1.0) as i32;
        let height = self.last_layout.total_height.max(1.0) as i32;
        let surface = ImageSurface::create(Format::ARgb32, width, height)
            .expect("Falha ao alocar buffer Cairo para a Dock");

        // 5. Draws the dock
        draw::paint(
            &surface,
            &self.last_layout,
            &self.widget.config,
            &self.icon_surface_cache,
            &mut self.font_system,
            &mut self.swash_cache,
            &mut self.glyph_cache,
        );

        surface
    }

    fn on_mouse_event(&mut self, event: MouseEvent) -> bool {
        let (x, y, is_left, is_middle) = match event {
            MouseEvent::Press((x, y), button) => (x, y, button == 272, button == 274),
            MouseEvent::Motion((x, y)) => (x, y, false, false),
            _ => return false,
        };

        if let Some(item) = self.last_layout.find_clicked_item(x, y) {
            if is_left {
                let _ = Command::new("niri")
                    .args([
                        "msg",
                        "action",
                        "focus-window",
                        "--id",
                        &item.id.to_string(),
                    ])
                    .spawn();
                return true;
            } else if is_middle {
                let _ = Command::new("niri")
                    .args([
                        "msg",
                        "action",
                        "close-window",
                        "--id",
                        &item.id.to_string(),
                    ])
                    .spawn();
                return true;
            }
        }

        false
    }
}

pub fn init_widget(ctx: &mut BoxTemporaryCtx, config: DockConfig) -> DockCtx {
    // redraw signal
    let (redraw_tx, mut redraw_rx) = mpsc::channel::<()>(1);

    let manager = backend::dock::niri::register_dock_listener(redraw_tx, &config);

    let waker = ctx.make_redraw_ping();

    let output = if let Some(output_data) = &ctx.builder.output.data::<OutputData>() {
        output_data.with_output_info(|info| info.name.clone().unwrap_or_default())
    } else {
        String::new()
    };

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        rt.block_on(async move {
            while redraw_rx.recv().await.is_some() {
                waker.ping();
            }
        });
    });

    DockCtx {
        output,
        widget: DockWidget::new(config),
        last_layout: DockLayout::default(),
        font_system: cosmic_text::FontSystem::new(),
        swash_cache: cosmic_text::SwashCache::new(),
        icon_surface_cache: HashMap::new(),
        glyph_cache: HashMap::new(),
        niri_manager: manager,
    }
}
