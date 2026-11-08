//! Subprocess execution. Every child gets its own process group, which xtask
//! ends on drop and on SIGINT/SIGTERM, so nothing outlives it.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::{Mutex, PoisonError};

use anyhow::{Context, Result};
use duct::Expression;

static RUNNING_GROUPS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

pub fn install_signal_handler() -> Result<()> {
    ctrlc::set_handler(|| {
        kill_running_groups();
        std::process::exit(130);
    })
    .context("installing the SIGINT/SIGTERM handler")
}

/// Runs `cargo`, streaming its output (standard error merged into standard
/// output) to our standard error as it arrives, and returns it with the exit
/// status.
pub fn run_cargo(args: &[String], env: &[(&'static str, String)]) -> Result<(ExitStatus, String)> {
    let mut expression = duct::cmd("cargo", args).dir(workspace_root());
    for (key, value) in env {
        expression = expression.env(key, value);
    }
    let reader = expression
        .stderr_to_stdout()
        .before_spawn(|command| {
            put_in_own_process_group(command);
            Ok(())
        })
        .unchecked()
        .reader()
        .context("starting subprocess")?;
    let _group = reader.pids().first().map(|&pid| GroupGuard::register(pid));
    let mut captured = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut stderr = std::io::stderr().lock();
    loop {
        let read = (&reader)
            .read(&mut chunk)
            .context("reading subprocess output")?;
        if read == 0 {
            break;
        }
        captured.extend_from_slice(&chunk[..read]);
        // A closed stderr must not hide the capture.
        let _ = stderr.write_all(&chunk[..read]);
    }
    let status = reader
        .try_wait()
        .context("waiting for subprocess")?
        .map(|output| output.status)
        .context("subprocess output ended before it exited")?;
    Ok((status, String::from_utf8_lossy(&captured).into_owned()))
}

/// Runs `git` in the workspace root and returns its standard output.
pub fn git(args: &[&str]) -> Result<String> {
    let output = run(duct::cmd("git", args)
        .dir(workspace_root())
        .stdout_capture())?;
    anyhow::ensure!(
        output.status.success(),
        "`git {}` failed ({})",
        args.join(" "),
        output.status
    );
    String::from_utf8(output.stdout).context("git printed non-UTF-8 output")
}

/// Whether `cargo <name>` resolves to an installed subcommand.
pub fn cargo_subcommand_exists(name: &str) -> Result<bool> {
    let probe = duct::cmd("cargo", [name, "--version"])
        .stdout_null()
        .stderr_null();
    Ok(run(probe)?.status.success())
}

fn run(expression: Expression) -> Result<Output> {
    let handle = expression
        .before_spawn(|command| {
            put_in_own_process_group(command);
            Ok(())
        })
        .unchecked()
        .start()
        .context("starting subprocess")?;
    let _group = handle.pids().first().map(|&pid| GroupGuard::register(pid));
    let output = handle.wait().context("waiting for subprocess")?;
    Ok(output.clone())
}

/// Owns a child's process group (its pgid is the child's pid).
struct GroupGuard(u32);

impl GroupGuard {
    fn register(pgid: u32) -> Self {
        lock_groups().push(pgid);
        Self(pgid)
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        lock_groups().retain(|&pgid| pgid != self.0);
        kill_group(self.0);
    }
}

fn lock_groups() -> std::sync::MutexGuard<'static, Vec<u32>> {
    RUNNING_GROUPS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn kill_running_groups() {
    for &pgid in lock_groups().iter() {
        kill_group(pgid);
    }
}

#[cfg(unix)]
fn put_in_own_process_group(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(unix)]
fn kill_group(pgid: u32) {
    let Ok(pgid) = libc::pid_t::try_from(pgid) else {
        return;
    };
    // SAFETY: kill(2) takes plain integers; a negative pid addresses the
    // group. ESRCH (group already gone) is the normal case and is ignored.
    unsafe {
        libc::kill(-pgid, libc::SIGKILL);
    }
}

// Windows delivers Ctrl-C to every process on the console, children included.
#[cfg(not(unix))]
fn put_in_own_process_group(_command: &mut std::process::Command) {}

#[cfg(not(unix))]
fn kill_group(_pgid: u32) {}
