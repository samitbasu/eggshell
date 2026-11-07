//! Subprocess execution. Every child gets its own process group, which xtask
//! ends on drop and on SIGINT/SIGTERM, so nothing outlives it.

use std::path::{Path, PathBuf};
use std::process::ExitStatus;
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

pub fn run_cargo(args: &[String], env: &[(&str, &str)]) -> Result<ExitStatus> {
    let mut expression = duct::cmd("cargo", args).dir(workspace_root());
    for (key, value) in env {
        expression = expression.env(key, value);
    }
    run(expression)
}

/// Whether `cargo <name>` resolves to an installed subcommand.
pub fn cargo_subcommand_exists(name: &str) -> Result<bool> {
    let probe = duct::cmd("cargo", [name, "--version"])
        .stdout_null()
        .stderr_null();
    Ok(run(probe)?.success())
}

fn run(expression: Expression) -> Result<ExitStatus> {
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
    Ok(output.status)
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
