use crate::{config::Config, judge::JudgeExecutor};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use theoj_common::error::{Context, Result};
use theoj_common::judge::{ApiToJudgeMessage, JudgeInfo, JudgeTask, JudgeToApiMessage};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn run(config: Config) -> Result<()> {
    let ws_url = config
        .api_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let ws_url = format!("{}/api/judge/ws", ws_url);

    loop {
        tracing::info!("Connecting to {}", ws_url);

        match connect_and_handle(&ws_url, &config).await {
            Ok(_) => {
                tracing::info!("Connection closed normally");
            }
            Err(e) => {
                tracing::error!("Connection error: {:?}", e);
            }
        }

        tracing::info!("Reconnecting in 5 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn connect_and_handle(url: &str, config: &Config) -> Result<()> {
    let (ws_stream, _) = connect_async(url)
        .await
        .context("Failed to connect to WebSocket")?;

    tracing::info!("WebSocket connected");

    let (mut write, mut read) = ws_stream.split();

    let executor = Arc::new(RwLock::new(JudgeExecutor::new(config.clone())));

    let private_key = theoj_common::auth::load_private_key(&config.private_key_path)
        .context("Failed to load private key")?;

    let timestamp = chrono::Utc::now().timestamp();
    let challenge = theoj_common::auth::create_challenge(&config.judge_id, timestamp);
    let signature = theoj_common::auth::sign_message(&private_key, challenge)
        .context("Failed to sign message")?;

    let register_msg = JudgeToApiMessage::Register(JudgeInfo {
        judge_id: config.judge_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        signature,
    });

    // send register
    let json = serde_json::to_string(&register_msg)?;
    write.send(Message::Text(json.into())).await?;

    tracing::info!("Registered");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<JudgeToApiMessage>();

    // send
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if write.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // heartbeat
    let tx_clone = tx.clone();
    let executor_clone = executor.clone();
    let heartbeat_send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(8));

        loop {
            interval.tick().await;

            let load = {
                let exec = executor_clone.read().await;
                exec.get_load().await
            };

            let _ = tx_clone.send(JudgeToApiMessage::Ping(load));
        }
    });

    // recv
    let executor_clone = executor.clone();
    let tx_clone = tx.clone();
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_message(&text, &executor_clone, &tx_clone).await {
                    tracing::error!("Failed to handle message: {:?}", e);
                }
            }
            Ok(Message::Close(_)) => {
                tracing::info!("Received close message");
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {:?}", e);
                break;
            }
            _ => {}
        }
    }

    heartbeat_send_task.abort();
    send_task.abort();

    Ok(())
}

async fn handle_message(
    text: &str,
    executor: &Arc<RwLock<JudgeExecutor>>,
    tx: &tokio::sync::mpsc::UnboundedSender<JudgeToApiMessage>,
) -> Result<()> {
    let msg: ApiToJudgeMessage = serde_json::from_str(text).context("Failed to parse message")?;

    match msg {
        ApiToJudgeMessage::Pong => {
            tracing::debug!("Received pong");
        }
        ApiToJudgeMessage::JudgeTask(JudgeTask {
            submission_id,
            lang,
            code,
            time_limit,
            memory_limit,
            test_cases,
        }) => {
            tracing::info!("Received judge task for submission {}", submission_id);

            let executor = executor.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                let mut exec = executor.write().await;
                exec.execute_task(
                    submission_id,
                    lang,
                    code,
                    time_limit,
                    memory_limit,
                    test_cases,
                    tx,
                )
                .await;
            });
        }
    }

    Ok(())
}
