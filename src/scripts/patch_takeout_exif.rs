use crate::child::Child;
use crate::list_files::list_files;
use anyhow::Context;
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use parking_lot::Mutex;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn patch_takeout_exif(input: PathBuf) -> anyhow::Result<()> {
    let files = list_files(&input)?;
    tracing::info!("Detected {} files", files.len());

    let info_by_media = Mutex::new(BTreeMap::new());
    files.into_par_iter().for_each(|file| {
        if let Err(error) = handle_file(&info_by_media, &file) {
            tracing::warn!("failed to handle {}: {}", file.display(), error);
        }
    });
    let info_by_media = info_by_media.into_inner();

    tracing::info!("Read information about {} media files", info_by_media.len());
    let mut counters: BTreeMap<_, i32> = BTreeMap::new();
    for (media, info) in info_by_media {
        *counters
            .entry((info.takeout_time.is_some(), info.exif_time.is_some()))
            .or_default() += 1;

        if let MediaInfo {
            file_readable: true,
            takeout_time: Some(takeout_time),
            exif_time: None,
        } = info
        {
            let takeout_time_str = takeout_time.format("%Y:%m:%d %H:%M:%S").to_string();

            let output = Child::new("exiftool")
                .args([
                    format!("-ModifyDate={}", takeout_time),
                    format!("-CreateDate={}", takeout_time),
                    format!("-DateTimeOriginal={}", takeout_time),
                ])
                .arg("-overwrite_original")
                .arg(&media)
                .capture_stdout()
                .run();

            match output {
                Ok(_) => {
                    tracing::info!("Patched {} with {}", media.display(), takeout_time_str);
                }
                Err(error) => {
                    tracing::error!("Failed to patch {}: {}", media.display(), error);
                }
            }
        }
    }
    for ((has_takeout, has_exif), num) in counters {
        tracing::info!(
            "has_takeout = {}, has_exif = {}, num = {}",
            has_takeout,
            has_exif,
            num
        );
    }

    Ok(())
}

#[derive(Debug, Default)]
struct MediaInfo {
    takeout_time: Option<NaiveDateTime>,
    file_readable: bool,
    exif_time: Option<NaiveDateTime>,
}

fn handle_file(
    info_by_media: &Mutex<BTreeMap<PathBuf, MediaInfo>>,
    file: &Path,
) -> anyhow::Result<()> {
    let ext = file
        .extension()
        .context("missing extension")?
        .to_ascii_lowercase();

    if ext == "json" {
        let info = read_takeout_info(file)?;
        if let Some((key, value)) = info {
            info_by_media.lock().entry(key).or_default().takeout_time = Some(value);
        }
    } else if ext == "jpg"
        || ext == "mp4"
        || ext == "mov"
        || ext == "png"
        || ext == "heic"
        || ext == "jpeg"
        || ext == "gif"
    {
        let exif_time = read_exif_info(file)?;
        let mut info_by_media = info_by_media.lock();
        let entry = info_by_media.entry(file.to_owned()).or_default();
        entry.file_readable = true;
        entry.exif_time = exif_time;
    } else {
        tracing::warn!("skipping file: {:?}", file);
    }

    Ok(())
}

fn read_takeout_info(file: &Path) -> anyhow::Result<Option<(PathBuf, NaiveDateTime)>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Metadata {
        title: String,
        photo_taken_time: Option<MetadataTime>,
    }

    #[derive(Deserialize)]
    struct MetadataTime {
        timestamp: String,
    }

    let contents = fs::read_to_string(file)?;
    let metadata: Metadata = serde_json::from_str(&contents)?;
    let parent = file.parent().context("missing parent")?.to_owned();
    let media_path = parent.join(metadata.title);

    let Some(metadata_time) = metadata.photo_taken_time else {
        return Ok(None);
    };

    let time = DateTime::<Utc>::from_timestamp(metadata_time.timestamp.parse()?, 0)
        .context("invalid timestamp")?
        .naive_local();

    Ok(Some((media_path, time)))
}

fn read_exif_info(file: &Path) -> anyhow::Result<Option<NaiveDateTime>> {
    let output = Child::new("exiftool")
        .args(["-DateTimeOriginal", "-json"])
        .arg(file)
        .capture_stdout()
        .capture_stderr()
        .run()?;

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ExifData {
        date_time_original: Option<String>,
    }

    let stdout = output.stdout()?;
    let datas = serde_json::from_str::<Vec<ExifData>>(&stdout)?;
    let data = datas.first().context("missing exif data")?;
    let Some(date_time_original) = &data.date_time_original else {
        return Ok(None);
    };
    let exif_time = NaiveDateTime::parse_from_str(date_time_original, "%Y:%m:%d %H:%M:%S")?;
    if exif_time.year() < 1900 || exif_time.year() > 2100 {
        return Ok(None);
    }

    Ok(Some(exif_time))
}
