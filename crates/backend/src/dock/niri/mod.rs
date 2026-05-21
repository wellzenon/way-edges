use crate::niri::connection::Event;
use config::def::widgets::wrapbox::dock::DockConfig;
use niri_ipc::{Window, Workspace};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, LazyLock, RwLock};
use tokio::sync::watch;

#[derive(Default)]
pub struct NiriDockState {
    pub windows: BTreeMap<u64, Window>,
    pub workspaces: BTreeMap<u64, Workspace>,
}

#[derive(Default)]
pub struct IconResolutionConfig {
    pub theme: Option<String>,
    pub fallback: Option<String>,
    pub size: f64,
}

pub static DOCK_STATE: LazyLock<Arc<RwLock<NiriDockState>>> =
    LazyLock::new(|| Arc::new(RwLock::new(NiriDockState::default())));

pub static DOCK_NOTIFIER: LazyLock<(watch::Sender<()>, watch::Receiver<()>)> =
    LazyLock::new(|| watch::channel(()));

pub static ICON_CONFIG: RwLock<IconResolutionConfig> = RwLock::new(IconResolutionConfig {
    theme: None,
    fallback: None,
    size: 48.0,
});

fn configure_icon_resolution(config: &DockConfig) {
    let mut icon_config = ICON_CONFIG.write().unwrap();
    icon_config.theme = config.window_button.icon_theme.clone();
    icon_config.fallback = config.window_button.icon_fallback.clone();
    icon_config.size = config.window_button.icon_size;
}

pub fn register_dock_listener(config: &DockConfig) {
    crate::dock::DOCK_ENABLED.store(true, Ordering::Relaxed);
    configure_icon_resolution(config);
    crate::niri::init_and_sync(true, true);
}

pub async fn process_event(e: Event) {
    let mut state_changed = false;

    {
        let mut state = DOCK_STATE.write().unwrap();

        match e {
            Event::WorkspaceActivated { id, focused } => {
                if let Some(output) = state.workspaces.get(&id).map(|w| w.output.clone()) {
                    for (ws_id, ws) in state.workspaces.iter_mut() {
                        if ws.output == output {
                            let should_be_active = *ws_id == id;
                            let should_be_focused = should_be_active && focused;

                            if ws.is_active != should_be_active
                                || ws.is_focused != should_be_focused
                            {
                                ws.is_active = should_be_active;
                                ws.is_focused = should_be_focused;
                            }
                        }
                    }
                    state_changed = true;
                }
            }
            Event::WorkspacesChanged { workspaces } => {
                state.workspaces.clear();
                state
                    .workspaces
                    .extend(workspaces.into_iter().map(|w| (w.id, w)));
                state_changed = true;
            }
            Event::WindowsChanged { windows } => {
                state.windows.clear();
                state.windows.extend(windows.into_iter().map(|w| (w.id, w)));
                state_changed = true;
            }
            Event::WindowLayoutsChanged { changes, .. } => {
                for (id, new_layout) in changes {
                    if let Some(win) = state.windows.get_mut(&id) {
                        if win.layout != new_layout {
                            win.layout = new_layout;
                        }
                    }
                }
                state_changed = true;
            }
            Event::WindowOpenedOrChanged { window } => {
                if window.is_focused {
                    state
                        .windows
                        .values_mut()
                        .for_each(|w| w.is_focused = false);
                }
                state.windows.insert(window.id, window);
                state_changed = true;
            }
            Event::WindowClosed { id } => {
                if state.windows.remove(&id).is_some() {
                    state_changed = true;
                }
            }
            Event::WindowFocusChanged { id } => {
                if let Some(focused_id) = id {
                    for (win_id, win) in state.windows.iter_mut() {
                        let should_be_focused = *win_id == focused_id;
                        if win.is_focused != should_be_focused {
                            win.is_focused = should_be_focused;
                        }
                    }
                    state_changed = true;
                }
            }
            Event::WindowUrgencyChanged { id, is_urgent } => {
                if let Some(win) = state.windows.get_mut(&id) {
                    if win.is_urgent != is_urgent {
                        win.is_urgent = is_urgent;
                        state_changed = true;
                    }
                }
            }
        }
    }

    // Dispara o pulso pelo canal Tokio apenas se houve alteração real
    if state_changed {
        let _ = DOCK_NOTIFIER.0.send(());
    }
}
