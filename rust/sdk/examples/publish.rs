use actyx_sdk::{
    app_id,
    service::{EventService, PublishEvent, PublishRequest},
    tags, ActyxClient, AppManifest, Payload,
};
use futures::{stream, FutureExt, Stream, StreamExt, TryStreamExt};
use url::Url;

fn counter() -> impl Stream<Item = i32> {
    stream::iter(0..).then(|i| futures_timer::Delay::new(std::time::Duration::from_secs(1)).map(move |()| i))
}

async fn mk_http_client() -> anyhow::Result<ActyxClient> {
    let app_manifest = AppManifest::trial(
        app_id!("com.example.actyx-publish"),
        "Publish Example".into(),
        "0.1.0".into(),
    )
    .unwrap();
    let url = Url::parse("http://localhost:4454").unwrap();
    ActyxClient::new(url, app_manifest).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = mk_http_client().await?;
    let mut results = counter().flat_map(|i| {
        let request = PublishRequest {
            data: vec![
                PublishEvent {
                    tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor1"),
                    payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                },
                PublishEvent {
                    tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor2"),
                    payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                },
            ],
        };
        service.publish(request).into_stream()
    });

    while let Some(res) = results.try_next().await? {
        println!("{}", serde_json::to_value(res)?);
    }
    Ok(())
}
