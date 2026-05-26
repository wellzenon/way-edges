use std::collections::{BTreeMap, HashMap};

use backend::niri::{IconCacheKey, IconStatus, ICON_CACHE};
use backend::tray::item::{Icon, IconHandle};
use cairo::{Format, ImageSurface};
use config::def::shared::NumMargins;
use config::def::widgets::wrapbox::dock::{DockConfig, ShowTitles};
use niri_ipc::{Window, Workspace};
use util::color::COLOR_WHITE;
use util::text::{draw_text, TextConfig};

use crate::wayland::app;

#[derive(Debug, Clone, Copy)]
pub struct MarginsF64 {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl From<&NumMargins> for MarginsF64 {
    fn from(m: &NumMargins) -> Self {
        Self {
            left: m.left as f64,
            right: m.right as f64,
            top: m.top as f64,
            bottom: m.bottom as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DockItem {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub icon_surface: Option<ImageSurface>,
    pub id: f64,
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub is_focused: bool,
}

impl DockItem {
    /// Verifica se a coordenada do mouse colide com a hitbox deste botão
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

#[derive(Debug, Clone)]
pub struct DockWorkspace {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub tag_surface: Option<ImageSurface>,
    pub is_focused: bool,
    pub output: Option<String>,
    pub items: Vec<DockItem>,

    pub height: f64,
}

impl DockWorkspace {
    /// Verifica se a coordenada do mouse colide com a hitbox deste botão
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

fn load_icon(
    icon: &IconCacheKey,
    cache: &mut HashMap<IconCacheKey, ImageSurface>,
) -> Option<ImageSurface> {
    // 1. Hit do Cache Frontend O(1)
    if let Some(surface) = cache.get(icon) {
        return Some(surface.clone());
    }

    // 2. Consulta ao Cache Global do Backend
    let backend_cache = ICON_CACHE.read().unwrap();
    let status = backend_cache.get(&icon)?;

    match status {
        IconStatus::Ready {
            pixels,
            width,
            height,
            ..
        } => {
            let mut surface = ImageSurface::create(Format::ARgb32, *width, *height).ok()?;

            {
                let stride = surface.stride() as usize;
                let mut data = surface.data().ok()?;
                let w_bytes = (*width * 4) as usize;

                for (dest_row, src_row) in data
                    .chunks_exact_mut(stride)
                    .zip(pixels.chunks_exact(w_bytes))
                {
                    dest_row[..w_bytes].copy_from_slice(src_row);
                }
            }
            surface.mark_dirty();

            cache.insert(icon.clone(), surface.clone());
            Some(surface)
        }
        IconStatus::Loading | IconStatus::NotFound => None,
    }
}

#[derive(Debug, Default, Clone)]
pub struct DockLayout {
    pub workspaces: Vec<DockWorkspace>,
    pub total_width: f64,
    pub total_height: f64,
}

impl DockLayout {
    /// O motor de roteamento espacial.
    /// Retorna a grade completa e as dimensões finais que o Cairo precisará alocar.
    pub fn calculate(
        windows: &[Window],
        workspaces: &BTreeMap<u64, Workspace>,
        config: &DockConfig,
        cache: &mut HashMap<IconCacheKey, ImageSurface>,
    ) -> Self {
        let mut current_x = 0.0;
        let ws_y = 0.0;

        let border_width = config.border_width as f64;
        let gap = config.gap as f64;
        let margins: MarginsF64 = (&config.margins).into();
        let workspace_titles = config.workspace_titles;

        let item_y = ws_y + border_width + margins.top;
        let item_border_width = config.window_button.border_width as f64;
        let item_gap = config.window_button.gap as f64;
        let icon_size = config.window_button.icon_size as f64;
        let icon_theme = &config.window_button.icon_theme;
        let icon_fallback = &config.window_button.icon_fallback;
        let show_titles = config.window_button.show_titles;
        let has_icon = show_titles != ShowTitles::Only && icon_size > 0.0;
        let title_width = config.window_button.title_width as f64;
        let item_margins: MarginsF64 = (&config.window_button.margins).into();

        let item_height =
            item_border_width * 2.0 + item_margins.top + item_margins.bottom + icon_size;

        let height = border_width * 2.0 + margins.top + margins.bottom + item_height;

        // SETUP DE MEDIÇÃO: Inicialização alocada apenas uma vez fora do loop
        let font_size = config.font_size as f32;

        let ws_vec: Vec<DockWorkspace> = windows
            .chunk_by(|a, b| a.workspace_id == b.workspace_id)
            .map(|ws| {
                let ws_id = ws[0].workspace_id.unwrap_or(0) as i32;

                let ws_x = current_x;
                current_x += border_width + margins.left;

                let (name, is_focused, output) = workspaces
                    .get(&(ws_id as u64))
                    .map(|ws| {
                        (
                            ws.name.clone().unwrap_or_else(|| ws.idx.to_string()),
                            ws.is_focused,
                            ws.output.clone(),
                        )
                    })
                    .unwrap_or_default();

                let tag_surface = if !name.is_empty() && workspace_titles {
                    let canvas = draw_text(
                        &name,
                        TextConfig::new(
                            config.font_family.as_family(),
                            None,
                            COLOR_WHITE,
                            font_size as i32,
                        ),
                    );

                    current_x += canvas.width as f64 + margins.left as f64;

                    Some(canvas.to_image_surface())
                } else {
                    None
                };

                let items: Vec<DockItem> = ws
                    .iter()
                    .map(|win| {
                        let has_title = match show_titles {
                            ShowTitles::Always | ShowTitles::Only => true,
                            ShowTitles::Focused if win.is_focused => true,
                            _ => false,
                        };

                        let mut item_width =
                            item_margins.left + item_margins.right + item_border_width * 2.0;

                        let app_id = win.app_id.clone().unwrap_or_default();

                        let icon = IconCacheKey {
                            app_id: app_id.clone(),
                            theme: icon_theme.clone(),
                            size: icon_size as u32,
                            fallback: icon_fallback.clone(),
                        };

                        let surface = if has_icon {
                            match load_icon(&icon, cache) {
                                Some(icon_surface) => {
                                    item_width += icon_surface.width() as f64;

                                    Some(icon_surface)
                                }
                                None => {
                                    let fallback = config.window_button.icon_fallback.as_deref();

                                    let identifier = if let Some(char_str) =
                                        fallback.filter(|f| f.chars().count() <= 4)
                                    {
                                        char_str.to_string()
                                    } else {
                                        let win_title = win
                                            .app_id
                                            .as_deref()
                                            .unwrap_or(win.title.as_deref().unwrap_or("?"));

                                        win_title
                                            .split('.')
                                            .last()
                                            .unwrap_or("?")
                                            .chars()
                                            .next()
                                            .unwrap_or('?')
                                            .to_uppercase()
                                            .to_string()
                                    };

                                    let canvas = draw_text(
                                        &identifier,
                                        TextConfig::new(
                                            config.window_button.font_family.as_family(),
                                            None,
                                            COLOR_WHITE,
                                            icon_size as i32,
                                        ),
                                    );

                                    item_width += canvas.width as f64;

                                    Some(canvas.to_image_surface())
                                }
                            }
                        } else {
                            None
                        };

                        if has_title {
                            item_width += title_width;
                        }

                        if has_icon && has_title {
                            item_width += item_margins.left;
                        }

                        let item_x = current_x;
                        current_x += item_width + item_gap;

                        DockItem {
                            x: item_x,
                            y: item_y,
                            width: item_width,
                            height: item_height,
                            icon_surface: surface,
                            id: win.id as f64,
                            app_id: win.app_id.clone(),
                            title: win.title.clone(),
                            is_focused: win.is_focused,
                        }
                    })
                    .collect();

                current_x -= item_gap;
                current_x += margins.right + border_width;

                let ws_width = current_x - ws_x;

                current_x += gap;

                DockWorkspace {
                    x: ws_x,
                    y: ws_y,
                    width: ws_width,
                    height,
                    tag_surface,
                    is_focused,
                    output,
                    items,
                }
            })
            .collect();

        current_x -= gap;

        DockLayout {
            workspaces: ws_vec,
            total_width: current_x,
            total_height: height,
        }
    }
    /// Usado no on_mouse_event para saber qual janela foi clicada
    pub fn find_clicked_item(&self, x: f64, y: f64) -> Option<&DockItem> {
        self.workspaces
            .iter()
            .find(|ws| ws.contains(x, y))
            .and_then(|ws| ws.items.iter().find(|item| item.contains(x, y)))
    }
}
