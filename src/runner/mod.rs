pub mod executor;
pub mod sudo;

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;

use anyhow::Result;

/// Whether subprocess stdout/stderr are only buffered or also streamed to the terminal while capturing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoMode {
    Buffered,
    LiveTee,
}

pub trait CommandRunner {
    fn run_with_env_io(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
        mode: IoMode,
    ) -> Result<std::process::Output>;

    fn run_with_stdin(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
        stdin_data: &str,
        mode: IoMode,
    ) -> Result<std::process::Output>;
}

pub struct LocalRunner;

impl CommandRunner for LocalRunner {
    fn run_with_env_io(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
        mode: IoMode,
    ) -> Result<std::process::Output> {
        match mode {
            IoMode::Buffered => {
                let mut command = Command::new(program);
                command.args(args);
                for &(key, value) in env {
                    command.env(key, value);
                }
                Ok(command.stdin(Stdio::inherit()).output()?)
            }
            IoMode::LiveTee => {
                let mut command = Command::new(program);
                command.args(args);
                for &(key, value) in env {
                    command.env(key, value);
                }
                command
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                let mut child = command.spawn()?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("missing child stdout"))?;
                let stderr = child
                    .stderr
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("missing child stderr"))?;

                let stdout_handle = thread::spawn(move || {
                    let mut lock = io::stdout().lock();
                    tee_reader(stdout, &mut lock)
                });
                let stderr_handle = thread::spawn(move || {
                    let mut lock = io::stderr().lock();
                    tee_reader(stderr, &mut lock)
                });

                let status = child.wait()?;

                let stdout_buf = stdout_handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("stdout tee thread panicked"))??;
                let stderr_buf = stderr_handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("stderr tee thread panicked"))??;

                Ok(std::process::Output {
                    status,
                    stdout: stdout_buf,
                    stderr: stderr_buf,
                })
            }
        }
    }

    fn run_with_stdin(
        &self,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
        stdin_data: &str,
        mode: IoMode,
    ) -> Result<std::process::Output> {
        match mode {
            IoMode::Buffered => {
                let mut command = Command::new(program);
                command.args(args);
                for &(key, value) in env {
                    command.env(key, value);
                }
                command.stdin(Stdio::piped());
                let mut child = command.spawn()?;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(stdin_data.as_bytes())?;
                }
                Ok(child.wait_with_output()?)
            }
            IoMode::LiveTee => {
                let mut command = Command::new(program);
                command.args(args);
                for &(key, value) in env {
                    command.env(key, value);
                }
                command
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                let mut child = command.spawn()?;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(stdin_data.as_bytes())?;
                }
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("missing child stdout"))?;
                let stderr = child
                    .stderr
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("missing child stderr"))?;

                let stdout_handle = thread::spawn(move || {
                    let mut lock = io::stdout().lock();
                    tee_reader(stdout, &mut lock)
                });
                let stderr_handle = thread::spawn(move || {
                    let mut lock = io::stderr().lock();
                    tee_reader(stderr, &mut lock)
                });

                let status = child.wait()?;

                let stdout_buf = stdout_handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("stdout tee thread panicked"))??;
                let stderr_buf = stderr_handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("stderr tee thread panicked"))??;

                Ok(std::process::Output {
                    status,
                    stdout: stdout_buf,
                    stderr: stderr_buf,
                })
            }
        }
    }
}

fn tee_reader<R: Read>(mut reader: R, sink: &mut impl Write) -> io::Result<Vec<u8>> {
    let mut accum = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sink.write_all(&buf[..n])?;
        sink.flush().ok();
        accum.extend_from_slice(&buf[..n]);
    }
    Ok(accum)
}
