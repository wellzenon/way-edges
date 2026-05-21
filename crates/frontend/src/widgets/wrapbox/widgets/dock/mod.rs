pub mod draw;
pub mod layout;

use crate::mouse_state::MouseEvent;
use crate::widgets::wrapbox::box_traits::BoxedWidget;
use crate::widgets::wrapbox::BoxTemporaryCtx;
use cairo::{Format, ImageSurface};
use config::def::widgets::wrapbox::dock::DockConfig;
use cosmic_text::{FontSystem, SwashCache};

use layout::DockLayout;
use std::{collections::HashMap, process::Command};

#[derive(Debug)]
pub struct DockWidget {
    pub config: DockConfig,
    // Futuramente, podemos guardar aqui o tamanho calculado do layout
    // ou estados de "hover" do mouse.
}

impl DockWidget {
    pub fn new(config: DockConfig) -> Self {
        Self { config }
    }
}

// 1. O Contexto do Widget (Exige Debug pelo trait)
#[derive(Debug)]
pub struct DockCtx {
    pub widget: DockWidget,
    pub last_layout: DockLayout, // <-- Guardamos a última matemática na memória
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub icon_surface_cache: std::collections::HashMap<String, ImageSurface>,
}

// 2. A IMPLEMENTAÇÃO DO MOTOR DE RENDERIZAÇÃO
impl BoxedWidget for DockCtx {
    // O Loop de Repintura
    fn content(&mut self) -> ImageSurface {
        // 1. Puxamos o estado instantâneo do nosso backend
        // (O try_read é perfeito aqui para não bloquear o loop de UI do Wayland)

        let workspaces = if let Ok(state) = backend::dock::niri::DOCK_STATE.try_read() {
            state.workspaces.values().cloned().collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let mut windows = if let Ok(state) = backend::dock::niri::DOCK_STATE.try_read() {
            state.windows.values().cloned().collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        windows.sort_by_key(|w| {
            (
                w.workspace_id,
                w.is_floating,
                w.layout.pos_in_scrolling_layout,
            )
        });

        // 2. Calculamos o Layout Matemático
        self.last_layout = DockLayout::calculate(windows, workspaces, &self.widget.config);

        // 3. Alocamos o Buffer de Memória da Placa de Vídeo
        let width = self.last_layout.total_width.max(1.0) as i32; // Evita falha do Cairo com largura 0
        let height = self.last_layout.total_height.max(1.0) as i32;
        // Passo 2: Alocar o Canvas de Memória (O Buffer Cairo)
        // Usamos ARgb32 para suportar transparência/Alpha.
        let surface = ImageSurface::create(Format::ARgb32, width, height)
            .expect("Falha ao alocar buffer Cairo para a Dock");

        // ==========================================================
        // O DESENHO REAL ACONTECERÁ AQUI:
        // let cr = cairo::Context::new(&surface).unwrap();
        // cr.set_source_rgba(0.0, 0.0, 0.0, 0.5); // Fundo escuro
        // cr.paint().unwrap();
        // (Delegaremos isso para o seu arquivo draw.rs)
        // ==========================================================

        draw::paint(
            &surface,
            &self.last_layout,
            &self.widget.config,
            &mut self.icon_surface_cache,
            &mut self.font_system,
            &mut self.swash_cache,
        );

        surface
    }

    fn on_mouse_event(&mut self, event: MouseEvent) -> bool {
        // Desestruturação exata baseada nas tuplas aninhadas do wrapbox do projeto
        let (x, y, is_left, is_middle) = match event {
            MouseEvent::Press((x, y), button) => {
                // 272 = clique esquerdo, 274 = clique do meio (padrão linux-input)
                (x, y, button == 272, button == 274)
            }
            MouseEvent::Motion((x, y)) => (x, y, false, false),
            _ => return false,
        };

        // Roteamento de hitboxes espaciais recalculadas pelo layout
        if let Some(item) = self.last_layout.find_clicked_item(x, y) {
            if is_left {
                let _ = Command::new("niri")
                    .args([
                        "msg",
                        "action",
                        "focus-window",
                        "--id",
                        &item.window.id.to_string(),
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
                        &item.window.id.to_string(),
                    ])
                    .spawn();
                return true;
            }
        }

        false
    }
}

// 3. A Inicialização
pub fn init_widget(ctx: &mut BoxTemporaryCtx, config: DockConfig) -> DockCtx {
    backend::dock::niri::register_dock_listener(&config);
    let mut rx = backend::dock::niri::DOCK_NOTIFIER.1.clone();

    // 1. Extraímos o "despertador" do way-edges
    let waker = ctx.make_redraw_ping(); // (Nome fictício, precisamos ver a struct)

    // 3. Spawna uma OS Thread dedicada (Descolada do Event Loop principal da UI)
    std::thread::spawn(move || {
        // Levanta um micro-runtime do Tokio de thread única (zero impacto de performance)
        // exclusivo para resolver a barreira assíncrona do canal `watch`.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Falha ao criar micro-runtime do Tokio para a Dock");

        // Bloqueia esta thread paralela aguardando os pulsos assíncronos
        rt.block_on(async move {
            while rx.changed().await.is_ok() {
                // Injeta o sinal no epoll da Main Thread (Wayland/Calloop)
                waker.ping();
            }
        });
    });

    // 5. Retorna o contexto instanciado
    DockCtx {
        widget: DockWidget::new(config),
        last_layout: DockLayout::default(),
        font_system: cosmic_text::FontSystem::new(),
        swash_cache: cosmic_text::SwashCache::new(),
        icon_surface_cache: HashMap::new(),
    }
}
