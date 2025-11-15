use crate::config::Config;
use std::sync::Arc;
use sysinfo::System;
use theoj_common::judge::{
    JudgeLoad, JudgeProgress, JudgeResult, JudgeToApiMessage, SubmissionResult, TestCase,
    TestCaseJudgeResult, TestCaseResult,
};
use tokio::sync::{RwLock, Semaphore};

pub struct JudgeExecutor {
    config: Config,
    running_tasks: Arc<RwLock<u32>>,
    semaphore: Arc<Semaphore>,

    system_info: Arc<RwLock<System>>,
    cached_load: Arc<RwLock<JudgeLoad>>,
}
impl JudgeExecutor {
    pub fn new(config: Config) -> Self {
        let executor = Self {
            config,
            running_tasks: Arc::new(RwLock::new(0)),
            semaphore: Arc::new(Semaphore::new(64)),
            system_info: Arc::new(RwLock::new(System::new_all())),
            cached_load: Arc::new(RwLock::new(JudgeLoad {
                running_tasks: 0,
                cpu_usage: 0.0,
                memory_usage: 0.0,
            })),
        };

        executor.spawn_load_updater();
        executor
    }
    fn spawn_load_updater(&self) {
        let system_info = self.system_info.clone();
        let cached_load = self.cached_load.clone();
        let running_tasks = self.running_tasks.clone();
        tokio::spawn(async move {
            loop {
                let mut sys = system_info.write().await;
                sys.refresh_cpu_all();
                sys.refresh_memory();

                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                sys.refresh_cpu_all();
                let cpu_usage = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                    / sys.cpus().len() as f32;
                let memory_usage = (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0;
                drop(sys);
                let running = *running_tasks.read().await;

                *cached_load.write().await = JudgeLoad {
                    running_tasks: running,
                    cpu_usage,
                    memory_usage,
                };
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    pub async fn get_load(&self) -> JudgeLoad {
        self.cached_load.read().await.clone()
    }

    pub async fn execute_task(
        &mut self,
        submission_id: i32,
        problem_id: i32,
        lang: String,
        code: String,
        time_limit: i32,
        memory_limit: i32,
        test_cases: Vec<TestCase>,
        tx: tokio::sync::mpsc::UnboundedSender<JudgeToApiMessage>,
    ) {
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        {
            let mut running = self.running_tasks.write().await;
            *running += 1;
        }

        let running_tasks = self.running_tasks.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let result = judge_submission(
                submission_id,
                problem_id,
                lang,
                code,
                time_limit,
                memory_limit,
                test_cases,
                &config,
                &tx,
            )
            .await;

            let _ = tx.send(result);

            {
                let mut running = running_tasks.write().await;
                *running -= 1;
            }

            drop(permit);
        });
    }
}

async fn judge_submission(
    submission_id: i32,
    problem_id: i32,
    lang: String,
    code: String,
    time_limit: i32,
    memory_limit: i32,
    test_cases: Vec<TestCase>,
    config: &Config,
    tx: &tokio::sync::mpsc::UnboundedSender<JudgeToApiMessage>,
) -> JudgeToApiMessage {
    tracing::info!(
        "Judging submission {} (problem {}, lang: {})",
        submission_id,
        problem_id,
        lang
    );

    let total_tests = test_cases.len();
    let mut test_results = Vec::new();
    let mut max_time = 0;
    let mut max_memory = 0;
    let mut final_result = SubmissionResult::Accepted;

    tracing::debug!("Compiling submission {}", submission_id);

    // compile

    // run tests
    for (idx, test_case) in test_cases.iter().enumerate() {
        tracing::debug!(
            "Running test case {} for submission {}",
            test_case.id,
            submission_id
        );

        // update prog
        let _ = tx.send(JudgeToApiMessage::JudgeProgress(JudgeProgress {
            submission_id,
            completed_tests: idx.try_into().unwrap(),
            total_tests: total_tests.try_into().unwrap(),
        }));

        // run test

        let time_used = 100; // ms
        let memory_used = 1024; // KB

        max_time = max_time.max(time_used);
        max_memory = max_memory.max(memory_used);

        let test_result = TestCaseResult {
            test_case_id: test_case.id,
            result: TestCaseJudgeResult::Accepted,
            time_consumption: time_used,
            memory_consumption: memory_used,
        };

        test_results.push(test_result);
    }

    // update prog
    let _ = tx.send(JudgeToApiMessage::JudgeProgress(JudgeProgress {
        submission_id,
        completed_tests: total_tests.try_into().unwrap(),
        total_tests: total_tests.try_into().unwrap(),
    }));

    tracing::debug!(
        "Submission {} judged: {:?}, time: {}ms, memory: {}KB",
        submission_id,
        final_result,
        max_time,
        max_memory
    );

    JudgeToApiMessage::JudgeResult(JudgeResult {
        submission_id,
        result: final_result,
        time_consumption: max_time,
        memory_consumption: max_memory,
        test_results,
    })
}
