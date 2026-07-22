use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::project::ProjectInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub project: Option<ProjectInfo>,
}

impl ProcessInfo {
    pub fn display_name(&self) -> &str {
        self.command.split_whitespace().next().unwrap_or("process")
    }
}

pub fn process_for_inode(inode: &str) -> Option<ProcessInfo> {
    for entry in fs::read_dir("/proc").ok()?.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if process_has_socket(pid, inode) {
            return Some(read_process(pid));
        }
    }

    None
}

fn process_has_socket(pid: u32, inode: &str) -> bool {
    let fd_dir = PathBuf::from(format!("/proc/{pid}/fd"));
    let expected = format!("socket:[{inode}]");

    let Ok(entries) = fs::read_dir(fd_dir) else {
        return false;
    };

    entries
        .flatten()
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .any(|target| target.to_string_lossy() == expected)
}

fn read_process(pid: u32) -> ProcessInfo {
    let command =
        read_cmdline(pid).unwrap_or_else(|| read_comm(pid).unwrap_or_else(|| format!("pid {pid}")));
    let cwd = fs::read_link(format!("/proc/{pid}/cwd")).ok();

    ProcessInfo {
        pid,
        command,
        cwd,
        project: None,
    }
}

fn read_cmdline(pid: u32) -> Option<String> {
    let bytes = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let parts = bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).to_string())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn read_comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn kill_process(pid: u32, force: bool) -> Result<(), String> {
    let signal = if force { "-KILL" } else { "-TERM" };
    let status = Command::new("kill")
        .arg(signal)
        .arg(pid.to_string())
        .status()
        .map_err(|error| format!("failed to run kill: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("kill exited with status {status}"))
    }
}

pub fn open_url(url: &str) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "xdg-open"
    };

    let mut command = Command::new(opener);
    if cfg!(target_os = "windows") {
        command.args(["/C", "start", "", url]);
    } else {
        command.arg(url);
    }

    command
        .spawn()
        .map_err(|error| format!("failed to open {url}: {error}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_uses_first_command_word() {
        let process = ProcessInfo {
            pid: 1,
            command: "npm run dev".to_string(),
            cwd: None,
            project: None,
        };

        assert_eq!(process.display_name(), "npm");
    }
}
