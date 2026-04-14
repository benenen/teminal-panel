use crate::project::{SshAuth, SshService};
use crate::terminal::{LocalShellFlavor, RemoteFileEntry};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteListCommandError {
    PasswordAuthUnsupported,
}

pub fn build_terminal_bootstrap_command(
    service: &SshService,
    remote_dir: &Path,
    shell_flavor: LocalShellFlavor,
) -> String {
    let args = build_terminal_bootstrap_args(service, remote_dir);

    match shell_flavor {
        LocalShellFlavor::Posix => render_posix_command(&args),
        LocalShellFlavor::Cmd => render_cmd_command(&args),
        LocalShellFlavor::PowerShell => render_powershell_command(&args),
    }
}

fn build_terminal_bootstrap_args(service: &SshService, remote_dir: &Path) -> Vec<String> {
    let mut args = vec!["ssh".to_string()];

    if service.port != 22 {
        args.push("-p".into());
        args.push(service.port.to_string());
    }

    if let SshAuth::Key { path, .. } = &service.auth {
        args.push("-i".into());
        args.push(path.display().to_string());
    }

    args.push(service.display_destination());
    args.push(format!(
        "cd {} && exec ${{SHELL:-/bin/bash}} -l",
        shell_quote_path(remote_dir)
    ));

    args
}

fn render_posix_command(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_quote_str(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_cmd_command(args: &[String]) -> String {
    args.iter()
        .map(|arg| cmd_quote_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_powershell_command(args: &[String]) -> String {
    let rendered = args
        .iter()
        .map(|arg| powershell_quote_arg(arg))
        .collect::<Vec<_>>()
        .join(" ");

    format!("& {rendered}")
}

pub fn build_remote_list_command(
    service: &SshService,
    remote_dir: &Path,
) -> Result<Vec<String>, RemoteListCommandError> {
    if matches!(service.auth, SshAuth::Password(_)) {
        return Err(RemoteListCommandError::PasswordAuthUnsupported);
    }

    let mut args = vec!["ssh".to_string()];

    if service.port != 22 {
        args.push("-p".into());
        args.push(service.port.to_string());
    }

    if let SshAuth::Key { path, .. } = &service.auth {
        args.push("-i".into());
        args.push(path.display().to_string());
    }

    args.push(service.display_destination());
    args.push(format!(
        "cd {} && for f in .[!.]* ..?* *; do [ -e \"$f\" ] || continue; if [ -d \"$f\" ]; then printf 'd\\t%s\\n' \"$f\"; else printf 'f\\t%s\\n' \"$f\"; fi; done",
        shell_quote_path(remote_dir)
    ));

    Ok(args)
}

pub fn load_remote_entries(
    service: &SshService,
    remote_dir: &Path,
) -> Result<Vec<RemoteFileEntry>, String> {
    let args = build_remote_list_command(service, remote_dir)
        .map_err(|_| "Remote browsing supports SSH agent/key auth only".to_string())?;

    let output = Command::new(&args[0])
        .args(&args[1..])
        .output()
        .map_err(|err| format!("Failed to run ssh: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "ssh command failed".into()
        } else {
            stderr
        });
    }

    parse_remote_entries(&String::from_utf8_lossy(&output.stdout), &remote_dir.display().to_string())
}

fn parse_remote_entries(text: &str, base_path: &str) -> Result<Vec<RemoteFileEntry>, String> {
    let mut entries = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (kind, name) = line
            .split_once('\t')
            .ok_or_else(|| format!("Invalid remote entry line: {line}"))?;

        entries.push(RemoteFileEntry {
            name: name.into(),
            path: format!("{}/{}", base_path.trim_end_matches('/'), name),
            is_dir: kind == "d",
        });
    }

    Ok(entries)
}

fn shell_quote_path(path: &Path) -> String {
    shell_quote_str(&path.display().to_string())
}

fn shell_quote_str(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn cmd_quote_arg(value: &str) -> String {
    let escaped = value
        .replace('"', r#"\""#)
        .replace('%', "%%")
        .replace('&', "^&")
        .replace('|', "^|")
        .replace('<', "^<")
        .replace('>', "^>")
        .replace('(', "^(")
        .replace(')', "^)");

    format!("\"{escaped}\"")
}

fn powershell_quote_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "`\"").replace('$', "`$"))
}

#[cfg(test)]
pub fn shell_quote_for_test(path: &Path) -> String {
    shell_quote_path(path)
}

#[cfg(test)]
pub fn parse_remote_entries_for_test(
    text: &str,
    base_path: &str,
) -> Result<Vec<RemoteFileEntry>, String> {
    parse_remote_entries(text, base_path)
}

