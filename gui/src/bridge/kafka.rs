use anyhow::Result;
use futures::StreamExt;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::Message,
    ClientConfig,
};
use tokio::sync::mpsc::Sender;

pub async fn run(broker: &str, topic: &str, tx: Sender<String>) -> Result<()> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("group.id", "flux-gui")
        .set("auto.offset.reset", "latest")
        .create()?;

    consumer.subscribe(&[topic])?;

    let mut stream = consumer.stream();
    while let Some(Ok(msg)) = stream.next().await {
        if let Some(Ok(payload)) = msg.payload_view::<str>() {
            tx.send(payload.to_string()).await.ok();
        }
    }
    Ok(())
}
