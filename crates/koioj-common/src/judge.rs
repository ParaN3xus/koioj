use core::fmt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, PartialOrd, Ord,
)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    C,
    Cpp,
    Java,
    Python,
    Go,
    Rust,
    JavaScript,
    TypeScript,
    CSharp,
    Php,
    Ruby,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Lua,
    Perl,
    R,
    Dart,
    ObjectiveC,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = serde_plain::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", s)
    }
}
impl FromStr for Language {
    type Err = serde_plain::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeInfo {
    pub judge_id: String,
    pub version: String,
    pub timestamp: i64,
    pub signature: String,
    pub languages: Vec<Language>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JudgeLoad {
    pub running_tasks: u32,
    pub cpu_usage: f32,    // 0.0 - 100.0
    pub memory_usage: f32, // 0.0 - 100.0
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum ApiToJudgeMessage {
    #[serde(rename = "judge_task")]
    JudgeTask(JudgeTask),
    #[serde(rename = "pong")]
    Pong,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JudgeTask {
    pub submission_id: i32,
    pub lang: Language,
    pub code: String,
    pub time_limit: i32,   // ms
    pub memory_limit: i32, // MB
    pub test_cases: Vec<TestCase>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TestCase {
    pub id: i32,
    pub data: TestCaseData,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseData {
    pub input: String,
    pub output: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
pub enum JudgeToApiMessage {
    #[serde(rename = "judge_result")]
    JudgeResult(JudgeResult),
    #[serde(rename = "judge_progress")]
    JudgeProgress(JudgeProgress),
    #[serde(rename = "ping")]
    Ping(JudgeLoad),
    #[serde(rename = "register")]
    Register(JudgeInfo),
    #[serde(rename = "error")]
    Error(i32, String),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JudgeResult {
    pub submission_id: i32,
    pub result: SubmissionResult,
    pub time_consumption: i32,   // ms
    pub memory_consumption: i32, // KB
    pub test_results: Vec<TestCaseResult>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JudgeProgress {
    pub submission_id: i32,
    pub completed_tests: u32,
    pub total_tests: u32,
}

#[derive(PartialEq, Clone, Copy, Debug, sqlx::Type, Serialize, Deserialize, ToSchema)]
#[sqlx(type_name = "submission_result_enum")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SubmissionResult {
    Pending,
    Accepted,
    WrongAnswer,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    RuntimeError,
    CompileError,
    UnknownError,
}

#[derive(PartialEq, Clone, Debug, sqlx::Type, Serialize, Deserialize, ToSchema)]
#[sqlx(type_name = "test_case_result_enum")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TestCaseJudgeResult {
    Pending,
    Compiling,
    Running,
    Accepted,
    WrongAnswer,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    RuntimeError,
    CompileError,
    UnknownError,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TestCaseResult {
    pub test_case_id: i32,
    pub result: TestCaseJudgeResult,
    pub time_consumption: i32,
    pub memory_consumption: i32,
}
