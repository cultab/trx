use crate::execute_external_command;
use ratatui::DefaultTerminal;
use std::collections::{HashMap, HashSet};
use std::process::Command;

use super::Package;

pub fn search_pacman(search_word: &str, query: &str) -> Vec<Package> {
    if search_word.trim().is_empty() {
        return Vec::new();
    }

    let output = Command::new("pacman").args(&["-Ss", search_word]).output();

    match output {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = output_str.lines().collect();
            super::parse_alternating_lines(&lines, "pacman".into(), query)
        }
        _ => Vec::new(),
    }
}

pub fn pacman_details(pkg: &str) -> Option<HashMap<String, String>> {
    let output = Command::new("pacman").args(&["-Si", pkg]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let s = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();

    for line in s.lines() {
        if let Some((k, v)) = line.split_once(" : ") {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    Some(map)
}

pub fn pacman_install(
    terminal: &mut DefaultTerminal,
    selected: &HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if selected.is_empty() {
        return Ok(());
    }

    let pure: Vec<String> = selected
        .iter()
        .map(|n| n.split('/').last().unwrap_or(n).to_string())
        .collect();

    let mut args: Vec<String> = vec!["pacman".into(), "-S".into()];
    args.extend(pure);

    let args_ref: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
    execute_external_command(terminal, "sudo", &args_ref)?;

    Ok(())
}
