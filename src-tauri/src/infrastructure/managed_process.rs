//! Owns a processor child until exit, including every handled failure path.
//!
//! Callers may hold a Mod mutation lease across `run`; parse failures, read
//! errors, cancellation and deadlines terminate and reap the child before
//! returning. Kill-on-drop also protects an unexpectedly abandoned future.
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};

const MAX_STDOUT_LINE: usize = 32 * 1024 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 16_000;
const CANCELLATION_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(crate) struct ProcessOutput {
    pub(crate) exit_code: Option<i32>,
    pub(crate) stderr: String,
}

pub(crate) fn command(program: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.as_std_mut().creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

fn append_diagnostic(output: &mut String, bytes: &[u8]) {
    for character in String::from_utf8_lossy(bytes).chars() {
        if output.len() + character.len_utf8() > MAX_DIAGNOSTIC_BYTES {
            break;
        }
        output.push(character);
    }
}

pub(crate) fn bounded_diagnostic(message: &str) -> String {
    let mut output = String::new();
    append_diagnostic(&mut output, message.as_bytes());
    output
}

pub(crate) async fn run(
    command: &mut Command,
    timeout: Option<Duration>,
    cancelled: impl Fn() -> bool,
    mut stdout_line: impl FnMut(&[u8]) -> Result<(), String>,
) -> Result<ProcessOutput, String> {
    if cancelled() {
        return Err("加工器运行已取消".into());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|error| format!("无法启动加工器：{error}"))?;
    // `command` configures both handles before spawn; no fallible return can
    // bypass the exit wait below once the child has been created.
    let mut stdout = child.stdout.take().expect("managed stdout is piped");
    let mut stderr = child.stderr.take().expect("managed stderr is piped");
    let mut out_chunk = [0u8; 8192];
    let mut err_chunk = [0u8; 8192];
    let mut line = Vec::new();
    let mut diagnostics = String::new();
    let mut stdout_open = true;
    let mut stderr_open = true;
    let mut exit_status = None;
    let mut tick = tokio::time::interval(CANCELLATION_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let deadline = timeout.map(|duration| tokio::time::Instant::now() + duration);

    let outcome: Result<(), String> = async {
        while exit_status.is_none() || stdout_open || stderr_open {
            tokio::select! {
                // A ready cancellation/deadline tick wins over an endlessly
                // ready stdout pipe, independently of processor output volume.
                biased;
                _ = tick.tick() => {
                    if cancelled() { return Err("加工器运行已取消".into()); }
                    if deadline.is_some_and(|deadline| tokio::time::Instant::now() >= deadline) {
                        return Err("加工器运行超时".into());
                    }
                }
                status = child.wait(), if exit_status.is_none() => {
                    exit_status = Some(status.map_err(|error| format!("等待加工器退出失败：{error}"))?);
                }
                result = stdout.read(&mut out_chunk), if stdout_open => {
                    let count = result.map_err(|error| format!("读取加工器输出失败：{error}"))?;
                    if count == 0 {
                        stdout_open = false;
                        if !line.is_empty() { stdout_line(&line)?; line.clear(); }
                    } else {
                        for part in out_chunk[..count].split_inclusive(|byte| *byte == b'\n') {
                            if line.len().saturating_add(part.len()) > MAX_STDOUT_LINE {
                                return Err("加工器单行输出超过大小限制".into());
                            }
                            line.extend_from_slice(part);
                            if line.last() == Some(&b'\n') {
                                line.pop();
                                if line.last() == Some(&b'\r') { line.pop(); }
                                stdout_line(&line)?;
                                line.clear();
                            }
                        }
                    }
                }
                result = stderr.read(&mut err_chunk), if stderr_open => {
                    let count = result.map_err(|error| format!("读取加工器错误输出失败：{error}"))?;
                    if count == 0 { stderr_open = false; }
                    else { append_diagnostic(&mut diagnostics, &err_chunk[..count]); }
                }
            }
        }
        Ok(())
    }.await;

    if let Err(reason) = outcome {
        let reason = bounded_diagnostic(&reason);
        if exit_status.is_none() {
            // Even if signalling fails, retain ownership until the actual exit
            // is observed. Returning early would release the caller's file lease.
            let signal_error = child.start_kill().err();
            child.wait().await.map_err(|wait_error| {
                format!("{reason}；无法确认加工器退出：{wait_error}；停止结果：{signal_error:?}")
            })?;
        }
        return Err(reason);
    }
    Ok(ProcessOutput {
        exit_code: exit_status.expect("process exit observed").code(),
        stderr: diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        path::PathBuf,
        sync::atomic::{AtomicBool, Ordering},
    };

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("hub-child-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn marker(&self) -> PathBuf {
            self.0.join("activity")
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // Runs harmless fixture code only when this exact test is selected in the
    // spawned copy of the current test executable. Ordinary suite runs return.
    #[test]
    fn child_process_fixture() {
        let Ok(mode) = std::env::var("D2RHUB_CHILD_FIXTURE") else {
            return;
        };
        let marker = PathBuf::from(std::env::var_os("D2RHUB_CHILD_MARKER").unwrap());
        let mut out = std::io::stdout().lock();
        let mut err = std::io::stderr().lock();
        std::fs::write(&marker, b"started").unwrap();
        if mode == "diagnostics" {
            write!(err, "{}", "错误".repeat(20_000)).unwrap();
            return;
        }
        if mode == "malformed" {
            writeln!(out, "{{\"type\":\"completed\",\"report\":null}}").unwrap();
            out.flush().unwrap();
        }
        for index in 0u64.. {
            std::fs::write(&marker, index.to_string()).unwrap();
            if mode == "flood" {
                for _ in 0..100 {
                    writeln!(out, "{{\"type\":\"progress\",\"percent\":1}}").unwrap();
                    writeln!(err, "diagnostic output that must remain bounded").unwrap();
                }
                out.flush().unwrap();
                err.flush().unwrap();
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn fixture(mode: &str, scratch: &Scratch) -> Command {
        let mut command = command(&std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "infrastructure::managed_process::tests::child_process_fixture",
                "--nocapture",
            ])
            .env("D2RHUB_CHILD_FIXTURE", mode)
            .env("D2RHUB_CHILD_MARKER", scratch.marker());
        command
    }

    fn assert_writes_stopped(scratch: &Scratch) {
        let after_exit = std::fs::read(scratch.marker()).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(std::fs::read(scratch.marker()).unwrap(), after_exit);
    }

    #[test]
    fn malformed_report_terminates_and_reaps_child_before_returning() {
        let scratch = Scratch::new();
        let result = tauri::async_runtime::block_on(run(
            &mut fixture("malformed", &scratch),
            Some(Duration::from_secs(5)),
            || false,
            |line| {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) {
                    if value["type"] == "completed" {
                        serde_json::from_value::<crate::domain::mod_processing::GeneratorReport>(
                            value["report"].clone(),
                        )
                        .map_err(|_| "malformed report".to_string())?;
                    }
                }
                Ok(())
            },
        ));
        assert!(result.err().unwrap().contains("malformed report"));
        assert_writes_stopped(&scratch);
    }

    #[test]
    fn continuous_output_cannot_starve_cancellation() {
        let scratch = Scratch::new();
        let cancel = AtomicBool::new(false);
        let mut lines = 0;
        let result = tauri::async_runtime::block_on(run(
            &mut fixture("flood", &scratch),
            Some(Duration::from_secs(5)),
            || cancel.load(Ordering::Acquire),
            |_| {
                lines += 1;
                if lines >= 100 {
                    cancel.store(true, Ordering::Release);
                }
                Ok(())
            },
        ));
        assert!(result.err().unwrap().contains("取消"));
        assert!(lines >= 100);
        assert_writes_stopped(&scratch);
    }

    #[test]
    fn timeout_terminates_and_reaps_a_silent_child() {
        let scratch = Scratch::new();
        let result = tauri::async_runtime::block_on(run(
            &mut fixture("silent", &scratch),
            Some(Duration::from_secs(2)),
            || false,
            |_| Ok(()),
        ));
        assert!(result.err().unwrap().contains("超时"));
        assert_writes_stopped(&scratch);
    }

    #[test]
    fn stderr_is_drained_to_completion_but_retained_diagnostics_stay_bounded() {
        let scratch = Scratch::new();
        let output = tauri::async_runtime::block_on(run(
            &mut fixture("diagnostics", &scratch),
            Some(Duration::from_secs(5)),
            || false,
            |_| Ok(()),
        ))
        .unwrap();
        assert_eq!(output.exit_code, Some(0));
        assert!(output.stderr.len() <= MAX_DIAGNOSTIC_BYTES);
        assert!(output.stderr.starts_with("错误"));
        assert!(output.stderr.ends_with('误') || output.stderr.ends_with('错'));
    }

    #[test]
    fn abandoning_the_supervisor_future_terminates_its_child() {
        let scratch = Scratch::new();
        tauri::async_runtime::block_on(async {
            let mut command = fixture("silent", &scratch);
            let supervisor =
                tokio::spawn(async move { run(&mut command, None, || false, |_| Ok(())).await });
            tokio::time::timeout(Duration::from_secs(5), async {
                while !scratch.marker().exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            supervisor.abort();
            assert!(supervisor.await.unwrap_err().is_cancelled());
            // The fallback Drop issues termination synchronously; the OS may
            // need a scheduling turn to finish it. Handled failures await exit.
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        assert_writes_stopped(&scratch);
    }
}
