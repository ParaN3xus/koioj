use std::{
    io::{Cursor, Read, Write},
    process::{Command, Stdio},
};

use koioj_common::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    Ok = 0,
    Tle = 1,
    Mle = 2,
    Re = 3,
    Uke = 5,
}

impl From<i32> for Verdict {
    fn from(v: i32) -> Self {
        match v {
            0 => Verdict::Ok,
            1 => Verdict::Tle,
            2 => Verdict::Mle,
            3 => Verdict::Re,
            _ => Verdict::Uke,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // stderr unused
pub struct JudgerResult {
    pub verdict: Verdict,
    pub time: i32,
    pub memory: i64,
    pub stdout: String,
    pub stderr: String,
    pub output_files: Vec<(String, Vec<u8>)>,
}

#[derive(Clone)]
pub struct FileInput {
    pub filename: String,
    pub content: Vec<u8>,
    pub mode: i32,
}

impl FileInput {
    pub fn text(filename: &str, content: &str, mode: i32) -> Self {
        FileInput {
            filename: filename.to_string(),
            content: content.as_bytes().to_vec(),
            mode,
        }
    }
}

fn write_i32(w: &mut impl Write, v: i32) -> Result<()> {
    w.write_all(&v.to_le_bytes())
        .map_err(|e| Error::anyhow(e.into()))
}

fn write_i64(w: &mut impl Write, v: i64) -> Result<()> {
    w.write_all(&v.to_le_bytes())
        .map_err(|e| Error::anyhow(e.into()))
}

fn write_str(w: &mut impl Write, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    write_i32(w, bytes.len() as i32)?;
    w.write_all(bytes).map_err(|e| Error::anyhow(e.into()))
}

fn read_i32(r: &mut impl Read) -> Result<i32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_i64(r: &mut impl Read) -> Result<i64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

fn read_string(r: &mut impl Read) -> Result<String> {
    let len = read_i32(r)?;
    let mut buf = vec![0u8; len as usize];
    if len > 0 {
        r.read_exact(&mut buf)?;
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn read_bytes(r: &mut impl Read) -> Result<Vec<u8>> {
    let len = read_i32(r)?;
    let mut buf = vec![0u8; len as usize];
    if len > 0 {
        r.read_exact(&mut buf)?;
    }
    Ok(buf)
}

pub fn run_judger(
    judger_bin_path: &str,
    rootfs: &str,
    tmpfs_size: &str,
    cgroup: &str,
    sandbox_id: &str,
    time_limit_ms: i32,
    memory_limit_mb: i64,
    pids_limit: i32,
    stdin_content: &str,
    cmdline: &[&str],
    files: &[FileInput],
    output_filenames: &[&str],
) -> Result<JudgerResult> {
    let mut child = Command::new(judger_bin_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    // write
    {
        let mut stdin = child.stdin.take().unwrap();

        write_i32(&mut stdin, time_limit_ms)?;
        write_i64(&mut stdin, memory_limit_mb)?;
        write_i32(&mut stdin, pids_limit)?;
        write_str(&mut stdin, rootfs)?;
        write_str(&mut stdin, tmpfs_size)?;
        write_str(&mut stdin, cgroup)?;
        write_str(&mut stdin, sandbox_id)?;
        write_str(&mut stdin, stdin_content)?;

        // cmdline
        write_i32(&mut stdin, cmdline.len() as i32)?;
        for s in cmdline {
            write_str(&mut stdin, s)?;
        }

        // input files
        write_i32(&mut stdin, files.len() as i32)?;
        for f in files {
            write_str(&mut stdin, &f.filename)?;
            write_i32(&mut stdin, f.content.len() as i32)?;
            stdin.write_all(&f.content)?;
            write_i32(&mut stdin, f.mode)?;
        }

        // output filenames
        write_i32(&mut stdin, output_filenames.len() as i32)?;
        for s in output_filenames {
            write_str(&mut stdin, s)?;
        }
    }

    // read output
    let output = child.wait_with_output()?;

    if !output.status.success() {
        return Err(Error::msg("Judger process exited abnormally"));
    }

    let mut cursor = Cursor::new(output.stdout);

    let verdict = Verdict::from(read_i32(&mut cursor)?);
    let time = read_i32(&mut cursor)?;
    let memory = read_i64(&mut cursor)?;
    let stdout = read_string(&mut cursor)?;
    let stderr = read_string(&mut cursor)?;

    let files_cnt = read_i32(&mut cursor)?;
    let mut output_files = Vec::with_capacity(files_cnt as usize);
    for _ in 0..files_cnt {
        let name = read_string(&mut cursor)?;
        let content = read_bytes(&mut cursor)?;
        output_files.push((name, content));
    }

    Ok(JudgerResult {
        verdict,
        time,
        memory,
        stdout,
        stderr,
        output_files,
    })
}

pub async fn run_judger_async(
    judger_bin_path: &str,
    rootfs: &str,
    tmpfs_size: &str,
    cgroup: &str,
    sandbox_id: &str,
    time_limit_ms: i32,
    memory_limit_mb: i64,
    pids_limit: i32,
    stdin_content: &str,
    cmdline: &[&str],
    files: &[FileInput],
    output_filenames: &[&str],
) -> Result<JudgerResult> {
    let judger_bin_path = judger_bin_path.to_string();
    let rootfs = rootfs.to_string();
    let tmpfs_size = tmpfs_size.to_string();
    let cgroup = cgroup.to_string();
    let sandbox_id = sandbox_id.to_string();
    let stdin_content = stdin_content.to_string();
    let cmdline: Vec<String> = cmdline.iter().map(|s| s.to_string()).collect();
    let files = files.to_vec();
    let output_filenames: Vec<String> = output_filenames.iter().map(|s| s.to_string()).collect();

    tokio::task::spawn_blocking(move || {
        let cmdline_refs: Vec<&str> = cmdline.iter().map(|s| s.as_str()).collect();
        let output_refs: Vec<&str> = output_filenames.iter().map(|s| s.as_str()).collect();

        run_judger(
            &judger_bin_path,
            &rootfs,
            &tmpfs_size,
            &cgroup,
            &sandbox_id,
            time_limit_ms,
            memory_limit_mb,
            pids_limit,
            &stdin_content,
            &cmdline_refs,
            &files,
            &output_refs,
        )
    })
    .await
    .map_err(|e| Error::anyhow(anyhow::anyhow!("Spawn blocking error: {}", e)))?
}
