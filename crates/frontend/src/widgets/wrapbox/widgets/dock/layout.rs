use config::def::{
    shared::NumMargins,
    widgets::wrapbox::dock::{DockConfig, ShowTitles},
};
use niri_ipc::{Window, Workspace};

#[derive(Debug, Clone)]
pub struct DockItem {
    pub window: Window,
    // Coordenadas absolutas dentro do buffer Cairo
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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
    pub height: f64,
    pub name: Option<String>,
    pub is_active: bool,
    pub items: Vec<DockItem>,
}

impl DockWorkspace {
    /// Verifica se a coordenada do mouse colide com a hitbox deste botão
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

#[allow(dead_code)]
struct Padding {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

struct BoxMetrics {
    height: f64,
    padding: Padding,
}

impl BoxMetrics {
    fn calculate(border_width: f64, margins: &NumMargins, content_height: f64) -> Self {
        let left = border_width + margins.left as f64;
        let right = border_width + margins.right as f64;
        let top = border_width + margins.top as f64;
        let bottom = border_width + margins.bottom as f64;

        let height = content_height + top + bottom;
        let padding = Padding {
            left,
            right,
            top,
            bottom,
        };

        Self { height, padding }
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
        windows: Vec<Window>,
        workspaces: Vec<Workspace>,
        config: &DockConfig,
    ) -> Self {
        let item_metrics = BoxMetrics::calculate(
            config.window_button.border_width,
            &config.window_button.margins,
            config.window_button.icon_size,
        );

        let dock_metrics = BoxMetrics::calculate(
            config.border_width as f64,
            &config.margins,
            item_metrics.height,
        );

        let ws_y = 0.0;
        let item_y = ws_y + dock_metrics.padding.top;

        let mut current_x = dock_metrics.padding.left;

        // Workspaces names width
        let ws_show_titles = config.workspace_titles;
        let ws_surface = cairo::ImageSurface::create(cairo::Format::A8, 1, 1).unwrap();
        let ws_cr = cairo::Context::new(&ws_surface).unwrap();
        let ws_font = match &config.window_button.font_family {
            cosmic_text::FamilyOwned::Name(nome) => nome.as_str(),
            cosmic_text::FamilyOwned::Serif => "serif",
            cosmic_text::FamilyOwned::SansSerif => "sans-serif",
            cosmic_text::FamilyOwned::Monospace => "monospace",
            cosmic_text::FamilyOwned::Cursive => "cursive",
            cosmic_text::FamilyOwned::Fantasy => "fantasy",
        };

        // 2. Configura EXATAMENTE a mesma fonte que será usada no desenho
        ws_cr.select_font_face(ws_font, cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        ws_cr.set_font_size(config.font_size as f64); // O tamanho da fonte da tag

        let ws_vec: Vec<DockWorkspace> = windows
            .chunk_by(|a, b| a.workspace_id == b.workspace_id)
            .map(|ws| {
                let ws_id = ws[0].workspace_id.unwrap_or(0) as i32;
                let mut is_active = false;

                let ws_x = current_x;

                let ws_name = if ws_show_titles {
                    let name = workspaces
                        .iter()
                        .find(|ws| ws.id as i32 == ws_id)
                        .map(|ws| ws.name.clone().unwrap_or_else(|| ws.idx.to_string()));

                    if let Some(ref text) = name {
                        // 3. Extrai as dimensões reais da string
                        let name_width = if let Ok(extents) = ws_cr.text_extents(text) {
                            extents.width()
                        } else {
                            0.0 // Fallback seguro
                        };
                        current_x += name_width + config.gap as f64;
                    }

                    name
                } else {
                    None
                };

                current_x += dock_metrics.padding.left;

                let items: Vec<DockItem> = ws
                    .iter()
                    .map(|win| {
                        let item_width = match config.window_button.show_titles {
                            ShowTitles::Always | ShowTitles::Focused if win.is_focused => {
                                config.window_button.icon_size
                                    + config.window_button.gap // gap between icon and text
                                    + config.window_button.title_width
                                    + item_metrics.padding.left
                                    + item_metrics.padding.right
                            } // gap between icon and text
                            _ => {
                                config.window_button.icon_size
                                    + item_metrics.padding.left
                                    + item_metrics.padding.right
                            }
                        };

                        let item_x = current_x;
                        current_x += item_width + config.gap as f64; // gap between items

                        is_active = is_active || win.is_focused;

                        DockItem {
                            window: win.clone(),
                            x: item_x,
                            y: item_y,
                            width: item_width,
                            height: item_metrics.height,
                        }
                    })
                    .collect();

                current_x -= config.gap as f64; // the last item has no gap
                current_x += dock_metrics.padding.right;

                let ws_width = current_x - ws_x;

                current_x += config.gap as f64; // gap between workspaces

                DockWorkspace {
                    x: ws_x,
                    y: ws_y,
                    width: ws_width,
                    height: dock_metrics.height,
                    name: ws_name,
                    is_active,
                    items,
                }
            })
            .collect();

        current_x -= config.gap as f64; // the last workspaces has no gap
        current_x += dock_metrics.padding.left;

        DockLayout {
            workspaces: ws_vec,
            total_width: current_x,
            total_height: dock_metrics.height,
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
