use anyhow::anyhow;
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use rand::Rng;
use std::{sync::Arc, time::Instant};
use theoj_common::judge::{
    ApiToJudgeMessage, JudgeInfo, JudgeLoad, JudgeTask, JudgeToApiMessage, Language,
    SubmissionResult, TestCaseJudgeResult,
};
use theoj_common::{bail, error::Context};
use tokio::sync::{RwLock, mpsc};

use crate::{AppState, Result, State, error::Error};

pub fn routes(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new().route("/ws", get(judge_ws))
}

#[derive(Clone)]
pub struct JudgeConnection {
    pub info: JudgeInfo,
    pub load: JudgeLoad,
    pub sender: mpsc::UnboundedSender<ApiToJudgeMessage>,
    pub last_heartbeat: Arc<RwLock<Instant>>,
}
impl JudgeConnection {
    pub fn load_score(&self) -> f32 {
        (self.load.running_tasks as f32) * 100.0
            + self.load.cpu_usage * 0.5
            + self.load.memory_usage * 0.3
    }
}

impl crate::AppState {
    pub async fn select_judge(&self, lang: Language) -> Result<String> {
        let judges = self.judges.read().await;

        if judges.is_empty() {
            bail!("no available judge");
        }

        // filter timeout judgers and language support
        let now = Instant::now();
        let mut available_judges = Vec::new();
        for (id, conn) in judges.iter() {
            let last_heartbeat = *conn.last_heartbeat.read().await;
            if now.duration_since(last_heartbeat).as_secs() < 60
                && conn.info.languages.contains(&lang)
            {
                available_judges.push((id, conn));
            }
        }

        if available_judges.is_empty() {
            bail!(
                "no available judge supporting {:?} (all timeout or language not supported)",
                lang
            );
        }

        // load
        let mut load_scores: Vec<(String, f32)> = available_judges
            .iter()
            .map(|(id, conn)| (id.to_string(), conn.load_score()))
            .collect();

        load_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let min_score = load_scores[0].1;

        // get candidates
        let threshold = min_score * 1.2;
        let candidates: Vec<String> = load_scores
            .iter()
            .filter(|(_, score)| *score <= threshold)
            .map(|(id, _)| id.clone())
            .collect();

        // random to prevent always the first
        let mut rng = rand::rng();
        let selected_idx = rng.random_range(0..candidates.len());

        Ok(candidates[selected_idx].clone())
    }

    pub async fn send_judge_task(&self, judge_id: &str, task: JudgeTask) -> Result<()> {
        let judges = self.judges.read().await;

        let conn = judges
            .get(judge_id)
            .ok_or_else(|| Error::msg(format!("judge not found: {}", judge_id)))?;

        conn.sender
            .send(ApiToJudgeMessage::JudgeTask(task))
            .map_err(|e| Error::msg(format!("failed to send task: {}", e)))?;

        Ok(())
    }

    pub async fn submit_judge_task(&self, task: JudgeTask) -> Result<()> {
        let judge_id = self.select_judge(task.lang).await?;
        self.send_judge_task(&judge_id, task).await
    }
}

#[utoipa::path(
    get,
    path = "/api/judge/ws",
    responses(
        (status = 101, description = "WebSocket connection established"),
    ),
    tag = "judge"
)]
pub async fn judge_ws(ws: WebSocketUpgrade, state: State) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: State) {
    let (mut sender, mut receiver) = socket.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<ApiToJudgeMessage>();

    let mut judge_id: Option<String> = None;
    let mut registered = false;

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        loop {
            let msg = receiver.next().await;
            match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Err(e) =
                        handle_judge_message(&text, &state, &mut judge_id, &mut registered, &tx)
                            .await
                    {
                        tracing::error!("Failed to handle judge message: {:?}", e);
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    break;
                }
                Some(Err(e)) => {
                    tracing::error!("WebSocket error: {:?}", e);
                    break;
                }
                _ => {}
            }
        }

        // cleaning
        if let Some(id) = judge_id {
            let mut judges = state.judges.write().await;
            judges.remove(&id);
            tracing::info!("Judge {} disconnected", id);
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
async fn handle_judge_message(
    text: &str,
    state: &State,
    judge_id: &mut Option<String>,
    registered: &mut bool,
    tx: &mpsc::UnboundedSender<ApiToJudgeMessage>,
) -> Result<()> {
    let msg: JudgeToApiMessage = serde_json::from_str(text)?;

    match msg {
        JudgeToApiMessage::Register(info) => {
            if *registered {
                tracing::warn!("Judge {} tried to register again", info.judge_id);
                return Ok(());
            }

            let key_path = state
                .config
                .judgers
                .get(&info.judge_id)
                .ok_or_else(|| Error::anyhow(anyhow!("Unknown judge_id: {}", info.judge_id)))?;

            let public_key = theoj_common::auth::load_public_key(&key_path)
                .context("Failed to load public key")?;

            let challenge = theoj_common::auth::create_challenge(&info.judge_id, info.timestamp);

            let sig_for_verify = info.signature.clone();
            theoj_common::auth::verify_signature(&public_key, challenge.as_bytes(), sig_for_verify)
                .context("Signature verification failed")?;

            let now = chrono::Utc::now().timestamp();
            let time_diff = (now - info.timestamp).abs();
            if time_diff > 60 {
                return Err(Error::anyhow(anyhow!(
                    "Timestamp too old or too new: {} seconds difference",
                    time_diff
                )));
            }

            tracing::info!(
                "Judge {} registered and verified, version: {}",
                info.judge_id,
                info.version
            );

            let conn = JudgeConnection {
                info: info.clone(),
                load: JudgeLoad {
                    running_tasks: 0,
                    cpu_usage: 0.0,
                    memory_usage: 0.0,
                },
                sender: tx.clone(),
                last_heartbeat: Arc::new(RwLock::new(Instant::now())),
            };

            let mut judges = state.judges.write().await;
            judges.insert(info.judge_id.clone(), conn);

            *judge_id = Some(info.judge_id);
            *registered = true;
        }

        JudgeToApiMessage::Ping(load) => {
            if !*registered {
                tracing::warn!("Received ping from unregistered judge");
                return Ok(());
            }

            if let Some(id) = judge_id {
                let mut judges = state.judges.write().await;
                if let Some(conn) = judges.get_mut(id) {
                    conn.load = load;
                    let mut last_heartbeat = conn.last_heartbeat.write().await;
                    *last_heartbeat = Instant::now();
                }
            }

            tx.send(ApiToJudgeMessage::Pong)?;
        }
        JudgeToApiMessage::JudgeProgress(progress) => {
            tracing::debug!(
                "Submission {} progress: {}/{}",
                progress.submission_id,
                progress.completed_tests,
                progress.total_tests
            );
            // TODO: somehow broadcast to frontend?
        }
        JudgeToApiMessage::JudgeResult(result) => {
            tracing::info!(
                "Submission {} result: {:?}, time: {}ms, memory: {}KB",
                result.submission_id,
                result.result,
                result.time_consumption,
                result.memory_consumption
            );

            let submission = sqlx::query!(
                r#"
                SELECT user_id, problem_id, contest_id, created_at
                FROM submissions
                WHERE id = $1
                "#,
                result.submission_id
            )
            .fetch_one(&state.pool)
            .await?;

            sqlx::query!(
                r#"
                UPDATE submissions 
                SET result = $1, time_consumption = $2, mem_consumption = $3, updated_at = NOW()
                WHERE id = $4
                "#,
                result.result as SubmissionResult,
                result.time_consumption,
                result.memory_consumption,
                result.submission_id
            )
            .execute(&state.pool)
            .await?;

            for test_result in result.test_results {
                sqlx::query!(
                    r#"
                    INSERT INTO submission_test_cases 
                    (submission_id, test_case_id, result, time_consumption, mem_consumption)
                    VALUES ($1, $2, $3, $4, $5)
                    "#,
                    result.submission_id,
                    test_result.test_case_id,
                    test_result.result as TestCaseJudgeResult,
                    test_result.time_consumption,
                    test_result.memory_consumption
                )
                .execute(&state.pool)
                .await?;
            }

            if let Some(contest_id) = submission.contest_id {
                if let Err(e) = crate::route::contests::ranking_cache::update_ranking_on_submission(
                    &state,
                    contest_id,
                    submission.user_id,
                    submission.problem_id,
                    result.result,
                    submission.created_at,
                )
                .await
                {
                    tracing::error!("Failed to update ranking cache: {}", e);
                    // Don't fail the whole operation if cache update fails
                }
            }
        }
        JudgeToApiMessage::Error(id, msg) => {
            tracing::error!("Submission {} judge error: {}", id, msg);

            // Get submission info to check if it's in a contest
            let submission = sqlx::query!(
                r#"
                SELECT user_id, problem_id, contest_id, created_at
                FROM submissions
                WHERE id = $1
                "#,
                id
            )
            .fetch_one(&state.pool)
            .await?;

            sqlx::query!(
                r#"
                UPDATE submissions 
                SET result = $1, time_consumption = $2, mem_consumption = $3, updated_at = NOW()
                WHERE id = $4
                "#,
                SubmissionResult::UnknownError as SubmissionResult,
                0,
                0,
                id
            )
            .execute(&state.pool)
            .await?;

            // Update ranking cache if this is a contest submission
            // UnknownError is treated as a failed attempt
            if let Some(contest_id) = submission.contest_id {
                if let Err(e) = crate::route::contests::ranking_cache::update_ranking_on_submission(
                    &state,
                    contest_id,
                    submission.user_id,
                    submission.problem_id,
                    SubmissionResult::UnknownError,
                    submission.created_at,
                )
                .await
                {
                    tracing::error!("Failed to update ranking cache: {}", e);
                    // Don't fail the whole operation if cache update fails
                }
            }
        }
    }

    Ok(())
}
