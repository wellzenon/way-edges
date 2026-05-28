use crate::niri::NiriManager;
use config::def::widgets::wrapbox::dock::DockConfig;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub fn register_dock_listener(
    redraw_tx: Sender<()>,
    config: &DockConfig,
) -> Option<Arc<NiriManager>> {
    crate::niri::init_and_sync(true, true, Some(redraw_tx), Some(config))
}
