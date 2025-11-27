use crate::config::Config;
use crate::judger::{FileInput, JudgerResult, run_judger};
use futures::future::join_all;
use std::sync::Arc;
use std::vec;
use sysinfo::System;
use theoj_common::judge::{
    JudgeLoad, JudgeResult, JudgeToApiMessage, Language, SubmissionResult, TestCase,
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
        lang: Language,
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
                lang,
                code,
                time_limit,
                memory_limit,
                test_cases,
                &config,
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
    lang: Language,
    code: String,
    time_limit: i32,
    memory_limit: i32,
    test_cases: Vec<TestCase>,
    config: &Config,
) -> JudgeToApiMessage {
    let lang_config = config.languages.get(&lang);

    let judger_bin_path = config.judger_bin_path.to_string_lossy().to_string();
    let rootfs_path = config.rootfs_path.to_string_lossy().to_string();
    let cgroup_base = config.cgroup_base.to_string_lossy().to_string();
    let tmpfs_size = "256M";
    let pids_limit = 16;

    if lang_config.is_none() {
        return JudgeToApiMessage::Error(submission_id, format!("Unsupported language {:?}", lang));
    }
    let lang_config = lang_config.unwrap();

    let compile_result: Option<JudgerResult>;

    // compile
    if let Some(compile_cmd) = &lang_config.compile {
        match run_judger(
            &judger_bin_path,
            &rootfs_path,
            tmpfs_size,
            &cgroup_base,
            &format!("theoj_judge_{}_compile", submission_id),
            5000,
            512,
            128,
            "",
            &compile_cmd
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>(),
            &[FileInput::text(&lang_config.source, &code, 0o644)],
            &[&lang_config.compiled],
        ) {
            Err(e) => {
                return JudgeToApiMessage::Error(
                    submission_id,
                    format!("Judger error when compiling: {:?}", e),
                );
            }
            Ok(res) if res.verdict == crate::judger::Verdict::Ok => {
                compile_result = Some(res);
            }
            Ok(res) => {
                tracing::debug!(
                    "Submission {} compile error: {:?}, time {}",
                    submission_id,
                    res.verdict,
                    res.time
                );
                return JudgeToApiMessage::JudgeResult(JudgeResult {
                    submission_id,
                    result: SubmissionResult::CompileError,
                    time_consumption: 0,
                    memory_consumption: 0,
                    test_results: vec![],
                });
            }
        }
    } else {
        compile_result = None;
    }

    // test
    let test_futures = test_cases.iter().map(|test_case| {
        let run_cmd = lang_config.run.clone();
        let compiled = lang_config.compiled.clone();
        let input = test_case.data.input.clone();
        let expected_output = test_case.data.output.clone();
        let test_id = test_case.id;
        let compile_result_ref = compile_result.as_ref();
        let rootfs_path = rootfs_path.clone();
        let judger_bin_path = judger_bin_path.clone();
        let cgroup_base = cgroup_base.clone();

        async move {
            let input_files: Vec<FileInput> = match compile_result_ref {
                Some(res) => match res.output_files.iter().find(|(name, _)| name == &compiled) {
                    Some((_, content)) => vec![FileInput {
                        filename: compiled,
                        content: content.to_vec(),
                        mode: 0o775,
                    }],
                    None => {
                        return TestCaseResult {
                            test_case_id: test_id,
                            result: TestCaseJudgeResult::UnknownError,
                            time_consumption: 0,
                            memory_consumption: 0,
                        };
                    }
                },
                None => vec![],
            };

            let run_result = run_judger(
                &judger_bin_path,
                &rootfs_path,
                tmpfs_size,
                &cgroup_base,
                &format!("theoj_judge_{}_test_{}", submission_id, test_id),
                time_limit.into(),
                memory_limit.into(),
                pids_limit,
                &input, // stdin
                &run_cmd.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
                &input_files,
                &[],
            );

            match run_result {
                Err(_) => TestCaseResult {
                    test_case_id: test_id,
                    result: TestCaseJudgeResult::UnknownError,
                    time_consumption: 0,
                    memory_consumption: 0,
                },
                Ok(res) => {
                    let result = match res.verdict {
                        crate::judger::Verdict::Ok => {
                            if res.stdout.trim() == expected_output.trim() {
                                TestCaseJudgeResult::Accepted
                            } else {
                                TestCaseJudgeResult::WrongAnswer
                            }
                        }
                        crate::judger::Verdict::Tle => TestCaseJudgeResult::TimeLimitExceeded,
                        crate::judger::Verdict::Mle => TestCaseJudgeResult::MemoryLimitExceeded,
                        crate::judger::Verdict::Re => TestCaseJudgeResult::RuntimeError,
                        _ => TestCaseJudgeResult::UnknownError,
                    };
                    TestCaseResult {
                        test_case_id: test_id,
                        result,
                        time_consumption: res.time,
                        memory_consumption: res.memory as i32,
                    }
                }
            }
        }
    });

    let test_results: Vec<TestCaseResult> = join_all(test_futures).await;

    let final_result = if test_results
        .iter()
        .all(|r| r.result == TestCaseJudgeResult::Accepted)
    {
        SubmissionResult::Accepted
    } else if test_results
        .iter()
        .any(|r| r.result == TestCaseJudgeResult::WrongAnswer)
    {
        SubmissionResult::WrongAnswer
    } else if test_results
        .iter()
        .any(|r| r.result == TestCaseJudgeResult::TimeLimitExceeded)
    {
        SubmissionResult::TimeLimitExceeded
    } else if test_results
        .iter()
        .any(|r| r.result == TestCaseJudgeResult::MemoryLimitExceeded)
    {
        SubmissionResult::MemoryLimitExceeded
    } else {
        SubmissionResult::RuntimeError
    };

    let total_time = test_results.iter().map(|r| r.time_consumption).sum();
    let max_memory = test_results
        .iter()
        .map(|r| r.memory_consumption)
        .max()
        .unwrap_or(0);

    JudgeToApiMessage::JudgeResult(JudgeResult {
        submission_id,
        result: final_result,
        time_consumption: total_time,
        memory_consumption: max_memory,
        test_results,
    })
}
