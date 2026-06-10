//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Process execution (`wxExecute`, `wxProcess`).

use std::process::{Child, Command, Output, Stdio};

/// Subprocess handle (`wxProcess`).
pub struct Process {
    child: Option<Child>,
}

impl Process {
    pub fn spawn(program: &str, args: &[&str]) -> std::io::Result<Self> {
        let child = Command::new(program).args(args).spawn()?;
        Ok(Self { child: Some(child) })
    }

    pub fn pid(&self) -> u32 {
        self.child.as_ref().map(|c| c.id()).unwrap_or(0)
    }

    pub fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        if let Some(child) = self.child.as_mut() {
            child.wait()
        } else {
            Ok(std::process::ExitStatus::default())
        }
    }

    pub fn wait_exit_event(&mut self) -> std::io::Result<crate::core::process_exit_event::ProcessExitEvent> {
        let pid = self.pid();
        let status = self.wait()?;
        Ok(crate::core::process_exit_event::ProcessExitEvent::new(
            pid,
            status.code().unwrap_or(-1),
        ))
    }
}

/// Synchronous execute (`wxExecute`).
pub fn execute(program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).output()
}

/// Detached launch with inherited stdio.
pub fn execute_async(program: &str, args: &[&str]) -> std::io::Result<Process> {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(Process { child: Some(child) })
}
