use anyhow::{Context, Result};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

pub async fn run(brokers: &str, topic: &str, tx: Sender<String>) -> Result<()> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", format!("flux-gui-{}", topic))
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "latest")
        .set("enable.auto.commit", "true")
        .set("session.timeout.ms", "6000")
        .create()
        .context("Failed to create Kafka consumer")?;

    consumer
        .subscribe(&[topic])
        .context("Failed to subscribe")?;

    let mut stream = consumer.stream();

    while let Some(result) = stream.next().await {
        if let Ok(msg) = result {
            if let Some(Ok(payload)) = msg.payload_view::<str>() {
                if tx.send(payload.to_owned()).await.is_err() {
                    break; // Channel closed, exit silently
                }
            }
        }
    }

    Ok(())
}
