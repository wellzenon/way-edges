pub mod niri;

use std::sync::atomic::AtomicBool;

// Flag global lida em Custo Zero.
// O frontend chamará DOCK_ENABLED.store(true) na sua inicialização.
pub static DOCK_ENABLED: AtomicBool = AtomicBool::new(false);

// Futuramente, você pode adicionar Traits genéricos aqui, como:
// pub trait DockProvider { fn get_windows() -> ... }
