// Concrete trigger submodules (feature-gated)
#[cfg(feature = "amqp_trigger")]
pub mod amqp;
#[cfg(feature = "http_trigger")]
pub mod http;
#[cfg(feature = "mqtt_trigger")]
pub mod mqtt;
#[cfg(feature = "postgres_trigger")]
pub mod postgres;
#[cfg(feature = "websocket")]
pub mod websocket;

// Assembly modules that stay local (reference concrete trigger types)
mod handler;
mod listener;

// Re-export everything from windmill-trigger base crate
pub use windmill_trigger::capture;
pub use windmill_trigger::filter;
pub use windmill_trigger::global_handler;
pub use windmill_trigger::trigger_helpers;
pub use windmill_trigger::types::*;

// Re-export traits
#[allow(unused)]
pub(crate) use windmill_trigger::Listener;
#[allow(unused)]
pub(crate) use windmill_trigger::TriggerCrud;

// Re-export windmill-trigger's handler module items needed by concrete triggers
pub use windmill_trigger::handler::{
    complete_trigger_routes, trigger_routes, TriggerPrimarySchedule,
};

// Re-export windmill-trigger's listener module items needed by concrete triggers
pub use windmill_trigger::listener::{
    listen_to_unlistened_events, listening, update_rw_lock, Capture, ListeningTrigger,
};

// Assembly functions (stay local because they reference concrete trigger types)
pub use handler::{generate_trigger_routers, get_triggers_count_internal, TriggersCount};
pub use listener::start_all_listeners;
