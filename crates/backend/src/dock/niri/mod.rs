use crate::niri::connection::Event;
use crate::niri::NiriManager;
use config::def::widgets::wrapbox::dock::DockConfig;
use niri_ipc::{Output, Window, Workspace};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, RwLock};
use tokio::sync::mpsc::Sender;

#[derive(Default)]
pub struct NiriDockState {
    pub windows: BTreeMap<u64, Window>,
    pub workspaces: BTreeMap<u64, Workspace>,

    // TODO golbal dock with all outputs odered by logical position
    pub outputs: BTreeMap<u64, Output>,
}

/// Dock Global state
pub static DOCK_STATE: LazyLock<Arc<RwLock<NiriDockState>>> =
    LazyLock::new(|| Arc::new(RwLock::new(NiriDockState::default())));

pub fn register_dock_listener(
    redraw_tx: Sender<()>,
    config: &DockConfig,
) -> Option<Arc<NiriManager>> {
    crate::niri::init_and_sync(true, true, Some(redraw_tx), Some(config))
}

pub async fn process_event(e: Event, redraw_tx: &Option<Sender<()>>) {
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

    if state_changed {
        if let Some(tx) = redraw_tx {
            let _ = tx.try_send(());
        }
    }
}
