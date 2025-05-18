use crate::list_files::list_files;
use anyhow::Context;
use csv::Writer;
use itertools::Itertools;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use stringmetrics::{LevWeights, levenshtein_weight};

pub fn match_deleted_films(
    rsync_log: PathBuf,
    sources: Vec<PathBuf>,
    already_matched: Option<PathBuf>,
    output: PathBuf,
) -> anyhow::Result<()> {
    let mut already_matched_sources = BTreeSet::new();
    let mut already_matched_targets = BTreeSet::new();
    if let Some(already_matched) = already_matched {
        let mut reader = csv::Reader::from_path(already_matched)?;
        for result in reader.deserialize() {
            let match_: Match = result?;
            if let Some(source) = match_.source {
                already_matched_sources.insert(source);
            }
            already_matched_targets.insert(match_.target);
        }
    }

    let mut source_files = vec![];
    for source in sources {
        for entry in list_files(&source)? {
            let source = SourceFile::new(entry)?;
            if !already_matched_sources.contains(&source.path) {
                source_files.push(source);
            }
        }
    }

    let target_files = fs::read_to_string(&rsync_log)?
        .lines()
        .filter_map(|line| {
            line.strip_prefix("deleting ")
                .filter(|line| !line.ends_with('/'))
        })
        .map(TargetFile::new)
        .filter(|target| !already_matched_targets.contains(&target.path))
        .collect_vec();

    tracing::info!(
        "Detected {} source and {} target files",
        source_files.len(),
        target_files.len()
    );

    // `distances[i][j]` is the edit distance from source `i` to target `j`
    let distances: Vec<_> = source_files
        .par_iter()
        .map(|source_file| {
            target_files
                .iter()
                .map(|target_file| distance(source_file, target_file))
                .collect_vec()
        })
        .collect();

    tracing::info!("Calculated distances");

    let mut matches = vec![];

    let mut pending_sources: BTreeSet<_> = (0..source_files.len()).collect();
    let mut pending_targets: BTreeSet<_> = (0..target_files.len()).collect();
    loop {
        let mut best_distance = u32::MAX;
        let mut best = None;
        for &pending_source in &pending_sources {
            for &pending_target in &pending_targets {
                let distance = distances[pending_source][pending_target];

                if distance < best_distance {
                    best_distance = distance;
                    best = Some((pending_source, pending_target));
                }
            }
        }

        let Some((best_source, best_target)) = best else {
            break;
        };
        pending_sources.remove(&best_source);
        pending_targets.remove(&best_target);

        matches.push(Match {
            source: Some(source_files[best_source].path.clone()),
            target: target_files[best_target].path.clone(),
            distance: Some(best_distance),
            done: None,
        });
    }

    for pending_target in pending_targets {
        matches.push(Match {
            source: None,
            target: target_files[pending_target].path.clone(),
            distance: None,
            done: None,
        })
    }
    matches.sort_by(|a, b| a.target.cmp(&b.target));

    tracing::info!("Found {} matches", matches.len());
    let mut writer = Writer::from_path(output)?;
    for match_ in matches {
        writer.serialize(match_).context("failed to write match")?;
    }
    writer.flush()?;

    Ok(())
}

#[derive(Debug)]
struct SourceFile {
    path: String,
    filename: String,
    extension: Option<String>,
    text: String,
    episode: Option<(u8, u8)>,
}

#[derive(Debug)]
struct TargetFile {
    path: String,
    filename: String,
    extension: Option<String>,
    text: String,
    episode: Option<(u8, u8)>,
}

impl SourceFile {
    fn new(path: PathBuf) -> anyhow::Result<Self> {
        let path = path.to_str().context("invalid file path")?.to_string();

        // Hacks
        let text = path
            .to_lowercase()
            .replace("FPP2011-compet", "Premiers Plans 2011");

        Ok(SourceFile {
            path,
            filename: filename(&text),
            extension: extension(&text),
            episode: episode(&text),
            text,
        })
    }
}

impl TargetFile {
    fn new(path: &str) -> Self {
        let path = path.to_string();

        let text = path.to_lowercase();

        TargetFile {
            path,
            filename: filename(&text),
            extension: extension(&text),
            episode: episode(&text),
            text,
        }
    }
}

fn filename(text: &str) -> String {
    let last = text.rsplit_once('/').unwrap().1;
    last.rsplit_once('.')
        .map(|(filename, _)| filename)
        .unwrap_or(last)
        .to_string()
}

fn extension(text: &str) -> Option<String> {
    text.rsplit_once('.').map(|(_, ext)| ext.to_lowercase())
}

fn episode(text: &str) -> Option<(u8, u8)> {
    static EPISODE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(?:S(\d+)E(\d+)| - (\d)x(\d+) - |Shinsekai_Yori_(\d+)_)").unwrap()
    });

    EPISODE_REGEX.captures(text).map(|captures| {
        let season = captures
            .get(1)
            .or_else(|| captures.get(3))
            .map(|c| c.as_str())
            .unwrap_or("1")
            .parse()
            .unwrap();
        let episode = captures
            .get(2)
            .or_else(|| captures.get(4))
            .or_else(|| captures.get(5))
            .unwrap()
            .as_str()
            .parse()
            .unwrap();
        (season, episode)
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Match {
    pub target: String,
    pub source: Option<String>,
    pub distance: Option<u32>,
    pub done: Option<bool>,
}

fn distance(source: &SourceFile, target: &TargetFile) -> u32 {
    // Modifying the filename is more expensive than the whole path
    // Deleting extra information from the filename and text is cheaper
    let filename_weights = LevWeights {
        insertion: 50,
        deletion: 5,
        substitution: 50,
    };
    let text_weights = LevWeights {
        insertion: 10,
        deletion: 1,
        substitution: 10,
    };

    if source.extension != target.extension {
        return u32::MAX;
    }

    let penalty = if source.episode != target.episode {
        1000
    } else {
        0
    };

    levenshtein_weight(
        &source.filename,
        &target.filename,
        u32::MAX,
        &filename_weights,
    ) + levenshtein_weight(&source.text, &target.text, u32::MAX, &text_weights)
        + penalty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(episode("Antoine.S06E05.1080p.blue.mkv").unwrap(), (6, 5));
    }
}
