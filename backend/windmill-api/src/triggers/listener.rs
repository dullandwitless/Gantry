use windmill_common::DB;
#[allow(unused_imports)]
use windmill_trigger::listener::{listen_to_unlistened_events, listening};
#[allow(unused_imports)]
use windmill_trigger::Listener;

// Re-export for backward compat with concrete trigger modules that use
// `crate::triggers::listener::ListeningTrigger`
#[allow(unused_imports)]
pub use windmill_trigger::listener::ListeningTrigger;

#[allow(unused)]
fn listen_to<T: Copy + Listener>(
    trigger: T,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        listen_to_unlistened_events(trigger, db.clone(), &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_events(trigger, db.clone(), &killpill_rx).await
                }
            }
        }
    });
}

#[allow(unused)]
pub fn start_all_listeners(db: DB, killpill_rx: &tokio::sync::broadcast::Receiver<()>) {
    tracing::info!("Starting trigger listeners based on available features...");

    #[cfg(feature = "postgres_trigger")]
    {
        let postgres_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::postgres::PostgresTrigger;

        listen_to(PostgresTrigger, db.clone(), postgres_killpill_rx)
    }

    #[cfg(feature = "mqtt_trigger")]
    {
        let mqtt_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::mqtt::MqttTrigger;

        listen_to(MqttTrigger, db.clone(), mqtt_killpill_rx)
    }

    #[cfg(feature = "amqp_trigger")]
    {
        let amqp_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::amqp::AmqpTrigger;

        listen_to(AmqpTrigger, db.clone(), amqp_killpill_rx)
    }

    #[cfg(feature = "websocket")]
    {
        let mqtt_killpill_rx = killpill_rx.resubscribe();
        use crate::triggers::websocket::WebsocketTrigger;

        listen_to(WebsocketTrigger, db.clone(), mqtt_killpill_rx)
    }

    tracing::info!("All available trigger listeners have been started");
}
