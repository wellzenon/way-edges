use std::collections::{BTreeMap, HashMap};

use backend::niri::IconKey;
use cairo::{ImageSurface, Rectangle};
use config::def::shared::NumMargins;
use config::def::widgets::wrapbox::dock::{DockConfig, ShowTitles};
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use niri_ipc::{Window, Workspace};

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
    pub rect: Rectangle,
    pub icon_rect: Rectangle,
    pub id: f64,
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub is_focused: bool,
    pub has_resolved_icon: bool,
    pub icon_key: IconKey,
    pub fallback_char: String,
}

impl DockItem {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.rect.x()
            && px <= (self.rect.x() + self.rect.width())
            && py >= self.rect.y()
            && py <= (self.rect.y() + self.rect.height())
    }
}

#[derive(Debug, Clone)]
pub struct DockWorkspace {
    pub rect: Rectangle,
    pub tag_rect: Rectangle,
    pub tag_name: Option<String>,
    pub is_focused: bool,
    pub items: Vec<DockItem>,

    // TODO future global dock with all outputs and ordered by logical position
    #[allow(dead_code)]
    pub output: Option<String>,
}

impl DockWorkspace {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.rect.x()
            && px <= (self.rect.x() + self.rect.width())
            && py >= self.rect.y()
            && py <= (self.rect.y() + self.rect.height())
    }
}

#[derive(Debug, Default, Clone)]
pub struct DockLayout {
    pub workspaces: Vec<DockWorkspace>,
    pub total_width: f64,
    pub total_height: f64,
}

impl DockLayout {
    pub fn calculate(
        windows: &[Window],
        workspaces: &BTreeMap<u64, Workspace>,
        config: &DockConfig,
        icon_surface_cache: &HashMap<IconKey, ImageSurface>,
        font_system: &mut FontSystem,
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
        let is_icon_enabled = show_titles != ShowTitles::Only && icon_size > 0.0;

        let title_width = config.window_button.title_width as f64;
        let item_margins: MarginsF64 = (&config.window_button.margins).into();

        let item_height =
            item_border_width * 2.0 + item_margins.top + item_margins.bottom + icon_size;

        let ws_height = border_width * 2.0 + margins.top + margins.bottom + item_height;
        let font_size = config.font_size as f32;

        let ws_vec: Vec<DockWorkspace> = windows
            .chunk_by(|a, b| a.workspace_id == b.workspace_id)
            .map(|ws| {
                let ws_id = ws[0].workspace_id.unwrap_or(0) as i32;
                let ws_x = current_x;

                current_x += border_width + margins.left;

                let (name, output) = workspaces
                    .get(&(ws_id as u64))
                    .map(|w| {
                        (
                            w.name.clone().or_else(|| Some(w.idx.to_string())),
                            w.output.clone(),
                        )
                    })
                    .unwrap_or_default();

                let tag_rect = if name.is_some() && workspace_titles {
                    let (width, _height) = measure_text(
                        font_system,
                        name.as_deref().unwrap(),
                        font_size,
                        config.font_family.as_family(),
                    );

                    let tag_rect = Rectangle::new(current_x, ws_y, width, ws_height);
                    current_x += width + margins.left as f64;

                    tag_rect
                } else {
                    Rectangle::new(0.0, 0.0, 0.0, 0.0)
                };

                // get ws focused from window focused
                let mut ws_focused = false;

                let items: Vec<DockItem> = ws
                    .iter()
                    .map(|win| {
                        // if the window focused is in this workspace the the workspace will be
                        // focused too
                        ws_focused = ws_focused || win.is_focused;

                        let has_title = match show_titles {
                            ShowTitles::Always | ShowTitles::Only => true,
                            ShowTitles::Focused if win.is_focused => true,
                            _ => false,
                        };

                        let mut item_width =
                            item_margins.left + item_margins.right + item_border_width * 2.0;

                        let app_id = win.app_id.clone().unwrap_or_default();

                        let icon_key = IconKey {
                            app_id: app_id.clone(),
                            theme: icon_theme.clone(),
                            size: icon_size as u32,
                            fallback: icon_fallback.clone(),
                        };

                        let (icon_rect, has_resolved_icon, fallback_char) = if is_icon_enabled {
                            let (icon_width, icon_height, has_resolved_icon) =
                                if let Some(surface) = icon_surface_cache.get(&icon_key) {
                                    (surface.width() as f64, surface.height() as f64, true)
                                } else {
                                    (icon_size, icon_size, false)
                                };

                            let icon_rec = Rectangle::new(
                                current_x + item_margins.left + item_border_width,
                                item_y + item_margins.top + item_border_width,
                                icon_width,
                                icon_height,
                            );

                            (icon_rec, has_resolved_icon, String::new())
                        } else {
                            let fallback_char = if let Some(fb) =
                                icon_fallback.as_deref().filter(|s| s.chars().count() == 1)
                            {
                                fb.to_string()
                            } else if let Some(title) =
                                win.title.as_deref().filter(|s| !s.is_empty())
                            {
                                title.chars().next().unwrap().to_uppercase().to_string()
                            } else {
                                String::new()
                            };

                            let (width, height) = measure_text(
                                font_system,
                                &fallback_char,
                                font_size,
                                config.font_family.as_family(),
                            );

                            let icon_rec = Rectangle::new(
                                current_x + item_margins.left + item_border_width,
                                item_y + item_margins.top + item_border_width,
                                width,
                                height,
                            );

                            (icon_rec, false, fallback_char)
                        };

                        item_width += icon_rect.width();

                        if has_title {
                            item_width += title_width;
                        }

                        if has_title && is_icon_enabled && icon_rect.width() > 0.0 {
                            item_width += item_margins.left;
                        }

                        let item_rect = Rectangle::new(current_x, item_y, item_width, item_height);
                        current_x += item_width + item_gap;

                        DockItem {
                            rect: item_rect,
                            icon_rect,
                            id: win.id as f64,
                            app_id: win.app_id.clone(),
                            title: win.title.clone(),
                            is_focused: win.is_focused,
                            has_resolved_icon,
                            icon_key,
                            fallback_char,
                        }
                    })
                    .collect();

                current_x -= item_gap;
                current_x += margins.right + border_width;

                let ws_width = current_x - ws_x;

                let ws_rect = Rectangle::new(ws_x, ws_y, ws_width, ws_height);

                current_x += gap;

                DockWorkspace {
                    rect: ws_rect,
                    tag_rect,
                    tag_name: name,
                    is_focused: ws_focused,
                    output,
                    items,
                }
            })
            .collect();

        current_x -= gap;

        DockLayout {
            workspaces: ws_vec,
            total_width: current_x,
            total_height: ws_height,
        }
    }

    pub fn find_clicked_item(&self, x: f64, y: f64) -> Option<&DockItem> {
        self.workspaces
            .iter()
            .find(|ws| ws.contains(x, y))
            .and_then(|ws| ws.items.iter().find(|item| item.contains(x, y)))
    }
}

pub fn measure_text(
    font_system: &mut FontSystem,
    text: &str,
    font_size: f32,
    font_family: Family,
) -> (f64, f64) {
    let metrics = Metrics::new(font_size, font_size);
    let mut buffer = Buffer::new(font_system, metrics);

    let attrs = Attrs::new().family(font_family);
    buffer.set_text(font_system, text, &attrs, Shaping::Advanced);
    buffer.shape_until_scroll(font_system, false);

    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let height = buffer.layout_runs().count() as f32 * font_size;

    (width as f64, height as f64)
}
