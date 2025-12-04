use super::Package;
use crate::execute_external_command;
use ratatui::DefaultTerminal;
use std::collections::{HashMap, HashSet};

use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Read;
use tar::Archive;

/*takes a data and just returns the next line

EXAMPLE:
%NAME%
firefox

so this takes NAME and desc as arguments and returns next line
*/
fn extract_field(desc: &str, key: &str) -> Option<String> {
    let marker = format!("%{}%\n", key);
    let start = desc.find(&marker)? + marker.len();
    let rest = &desc[start..];
    let end = rest.find('\n').unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

pub fn search_pacman(search_word: &str) -> Vec<Package> {
    let repos = [
        "/var/lib/pacman/sync/core.db",
        "/var/lib/pacman/sync/extra.db",
        "/var/lib/pacman/sync/multilib.db",
    ];

    let mut results: Vec<Package> = Vec::new();

    for repo in repos {
        let file = match File::open(repo) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let gz = GzDecoder::new(file);
        let mut ar = Archive::new(gz);

        let entries = match ar.entries() {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries {
            let mut e = match entry {
                Ok(x) => x,
                Err(_) => continue,
            };

            let path = match e.path() {
                Ok(p) => p.into_owned(),
                Err(_) => continue,
            };

            if !path.ends_with("desc") {
                continue;
            }

            let mut desc = String::new();
            if e.read_to_string(&mut desc).is_err() {
                continue;
            }

            // extraction of meta data
            let name = extract_field(&desc, "NAME");
            let version = extract_field(&desc, "VERSION");
            let description = extract_field(&desc, "DESC");

            let repo_name = repo
                .split('/')
                .last()
                .unwrap()
                .trim_end_matches(".db")
                .to_string();
            if let (Some(name), Some(version), Some(description)) = (name, version, description) {
                let score = crate::fuzzy::fuzzy_match(search_word, &name);

                if score > 0.01 {
                    results.push(Package {
                        provider: format!("pacman/{}", repo_name),
                        name,
                        version,
                        description,
                        score,
                    });
                }
            }
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

//core.db -> gzip -> tar -> pkgname/desc
//It makes a HashMap of fields and map them to values
pub fn pacman_details(pkg: &str) -> Option<HashMap<String, String>> {
    let repos = [
        ("core", "/var/lib/pacman/sync/core.db"),
        ("extra", "/var/lib/pacman/sync/extra.db"),
        ("multilib", "/var/lib/pacman/sync/multilib.db"),
    ];

    let pure = pkg.split('/').last().unwrap_or(pkg);

    for (repo_name, repo_path) in repos {
        let file = File::open(repo_path).ok()?;
        let gz = GzDecoder::new(file);
        let mut ar = Archive::new(gz);

        for entry in ar.entries().ok()? {
            let mut e = entry.ok()?;

            let path = e.path().ok()?.into_owned();
            if !path.ends_with("desc") {
                continue;
            }

            let mut text = String::new();
            e.read_to_string(&mut text).ok()?;

            // Parse pacman's desc format
            let mut raw = HashMap::<String, String>::new();
            let mut key = String::new();
            let mut buf = String::new();

            for line in text.lines() {
                if line.starts_with('%') {
                    if !key.is_empty() && !buf.is_empty() {
                        raw.insert(key.clone(), buf.trim().to_string());
                        buf.clear();
                    }
                    key = line.trim_matches('%').to_string();
                } else if !line.trim().is_empty() {
                    buf.push_str(line.trim());
                    buf.push('\n');
                }
            }
            if !key.is_empty() && !buf.is_empty() {
                raw.insert(key.clone(), buf.trim().to_string());
            }

            let name = raw.get("NAME")?;
            let fname = raw
                .get("FILENAME")
                .map(|f| f.trim_end_matches(".pkg.tar.zst"));

            let matched = name == pure
                || fname == Some(pure)
                || fname
                    .map(|f| f.starts_with(&format!("{}-", pure)))
                    .unwrap_or(false);

            if !matched {
                continue;
            }

            let mut out = HashMap::new();

            out.insert("Repository".into(), repo_name.into());
            out.insert("Name".into(), raw.get("NAME").cloned().unwrap_or_default());
            out.insert(
                "Version".into(),
                raw.get("VERSION").cloned().unwrap_or_default(),
            );
            out.insert(
                "Description".into(),
                raw.get("DESC").cloned().unwrap_or_default(),
            );
            out.insert(
                "Architecture".into(),
                raw.get("ARCH").cloned().unwrap_or_default(),
            );
            out.insert("URL".into(), raw.get("URL").cloned().unwrap_or_default());
            out.insert(
                "Licenses".into(),
                raw.get("LICENSE").cloned().unwrap_or_default(),
            );
            out.insert(
                "Groups".into(),
                raw.get("GROUPS").cloned().unwrap_or_default(),
            );
            out.insert(
                "Provides".into(),
                raw.get("PROVIDES").cloned().unwrap_or_default(),
            );
            out.insert(
                "Depends On".into(),
                raw.get("DEPENDS").cloned().unwrap_or_default(),
            );
            out.insert(
                "Optional Deps".into(),
                raw.get("OPTDEPENDS").cloned().unwrap_or_default(),
            );
            out.insert(
                "Conflicts".into(),
                raw.get("CONFLICTS").cloned().unwrap_or_default(),
            );
            out.insert(
                "Replaces".into(),
                raw.get("REPLACES").cloned().unwrap_or_default(),
            );
            out.insert(
                "Download Size".into(),
                raw.get("CSIZE").cloned().unwrap_or_default(),
            );
            out.insert(
                "Installed Size".into(),
                raw.get("ISIZE").cloned().unwrap_or_default(),
            );
            out.insert(
                "Packager".into(),
                raw.get("PACKAGER").cloned().unwrap_or_default(),
            );

            if let Some(ts) = raw.get("BUILDDATE") {
                out.insert("Build Date".into(), ts.clone());
            }

            out.insert(
                "Filename".into(),
                raw.get("FILENAME").cloned().unwrap_or_default(),
            );

            return Some(out);
        }
    }

    None
}

//uses normal pacman command
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
