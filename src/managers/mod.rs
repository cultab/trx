pub mod pacman;
pub mod yay;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub provider: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub score: f64,
}

//makes a DETAILS_CACHE which is global
lazy_static::lazy_static! {
    pub static ref DETAILS_CACHE: Arc<Mutex<HashMap<String, HashMap<String, String>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub fn parse_alternating_lines(lines: &[&str], manager: String, query: &str) -> Vec<Package> {
    let mut res = Vec::new();
    let mut i = 0;

    while i + 1 < lines.len() {
        let first_line = lines[i];
        let second_line = lines[i + 1];

        let parts: Vec<&str> = first_line.split_whitespace().collect();

        if parts.len() >= 2 {
            let package = parts[0].to_string();
            let version = parts[1].to_string();
            let description = second_line.trim().to_string();

            let package_name = package.split('/').last().unwrap_or(&package).to_string();
            let score = crate::fuzzy::fuzzy_match(query, &package_name);

            res.push(Package {
                provider: manager.clone(),
                name: package,
                version,
                description,
                score,
            });
        }

        i += 2;
    }

    res.retain(|p| p.score > 0.01);
    res.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    res
}

pub fn details_package(package: &str, provider: &str) -> Option<HashMap<String, String>> {
    {
        let cache = DETAILS_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(package) {
            return Some(cached.clone());
        }
    }

    let pure_name = package.split('/').last().unwrap();
    let provide = provider.split('/').next().unwrap();
    let info = match provide {
        "aur" => yay::aur_details(pure_name)?,
        "pacman" => pacman::pacman_details(pure_name)?,
        _ => return None,
    };

    let mut cache = DETAILS_CACHE.lock().unwrap();
    cache.insert(package.to_string(), info.clone());

    Some(info)
}
