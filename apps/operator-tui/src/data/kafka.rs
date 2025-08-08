use anyhow::{anyhow, Result};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

pub async fn run_with_retry(brokers: &str, topic: &str, tx: Sender<String>) -> Result<()> {
    macro_rules! bail {
        ($($arg:tt)*) => {{
            eprintln!("[KAFKA] {}", format!($($arg)*));
            return Ok(());
        }};
    }

    // --- Build Consumer ---
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", format!("flux-gui-{}", topic))
        .set("auto.offset.reset", "latest") // start at the live tail
        .create()
        .map_err(|e| anyhow!("cannot create consumer: {e}"))?;

    // --- Subscribe ---
    consumer
        .subscribe(&[topic])
        .map_err(|e| anyhow!("subscribe failed: {e}"))?;

    // DEBUG
    std::env::set_var("RDKAFKA_LOG_LEVEL", "6");

    // --- Stream Loop ---
    let mut stream = consumer.stream();
    loop {
        match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
            Ok(Some(Ok(msg))) => {
                if let Some(Ok(text)) = msg.payload_view::<str>() {
                    if tx.send(text.to_owned()).await.is_err() {
                        bail!("channel closed");
                    }
                } else if let Some(payload) = msg.payload() {
                    // Fallback if the payload wasn’t valid UTF-8 (rare, but log it)
                    eprintln!("[KAFKA] non-utf8 payload ({} bytes)", payload.len());
                }
            }
            Ok(Some(Err(e))) => bail!("message error: {e}"),
            Ok(None) => bail!("stream ended"),
            Err(_) => { /* 1 s idle → keep polling */ }
        }
    }
}

// Keep the old public symbol alive for anything else that calls it
pub async fn run(b: &str, t: &str, tx: Sender<String>) -> Result<()> {
    run_with_retry(b, t, tx).await
}
