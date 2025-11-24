use crate::execute_external_command;
use ratatui::DefaultTerminal;
use std::collections::{HashMap, HashSet};
use std::process::Command;

use super::Package;

fn normalize_aur_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a
            .iter()
            .map(|x| x.as_str().unwrap_or(&x.to_string()).to_string())
            .collect::<Vec<_>>()
            .join(", "),
        serde_json::Value::Null => "None".into(),
        _ => v.to_string(),
    }
}

pub fn search_aur(search_word: &str, query: &str) -> Vec<Package> {
    if search_word.trim().is_empty() {
        return Vec::new();
    }

    let output = Command::new("yay").args(&["-Ss", search_word]).output();

    match output {
        Ok(output) if output.status.success() => {
            let s = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = s.lines().collect();
            super::parse_alternating_lines(&lines, "aur".into(), query)
        }
        _ => Vec::new(),
    }
}

pub fn aur_details(pkg: &str) -> Option<HashMap<String, String>> {
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=info&arg={}", pkg);

    let resp = reqwest::blocking::get(url).ok()?;
    let json: serde_json::Value = resp.json().ok()?;

    let obj = if json["results"].is_array() {
        json["results"][0].as_object()?
    } else {
        json["results"].as_object()?
    };

    let mut out = HashMap::new();

    let mapping = [
        ("Name", "Name"),
        ("Version", "Version"),
        ("Description", "Description"),
        ("URL", "URL"),
        ("License", "Licenses"),
        ("Depends", "Depends On"),
        ("OptDepends", "Optional Deps"),
        ("MakeDepends", "Make Deps"),
        ("Conflicts", "Conflicts With"),
        ("Provides", "Provides"),
        ("Replaces", "Replaces"),
        ("Keywords", "Keywords"),
        ("Submitter", "Submitter"),
        ("Maintainer", "Maintainer"),
        ("Popularity", "Popularity"),
        ("NumVotes", "Votes"),
        ("FirstSubmitted", "First Submitted"),
        ("LastModified", "Last Modified"),
    ];

    for (aur_key, pacman_key) in mapping {
        if let Some(v) = obj.get(aur_key) {
            out.insert(pacman_key.into(), normalize_aur_value(v));
        }
    }

    Some(out)
}

pub fn aur_install(
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

    let mut args: Vec<String> = vec!["yay".into(), "-S".into()];
    args.extend(pure);

    let args_ref: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
    execute_external_command(terminal, "sudo", &args_ref)?;

    Ok(())
}
