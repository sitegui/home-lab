use crate::child::Child;
use crate::list_files::list_files;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub fn detect_films(path: &Path, output: &Path) -> anyhow::Result<()> {
    let mut film_paths = Vec::new();
    list_files(path, &mut film_paths)?;

    tracing::info!("Found {} files", film_paths.len());

    let mut writer = csv::Writer::from_path(output)?;
    let valid_extensions = [
        "avi", "flv", "m4v", "mkv", "mov", "mp3", "mp4", "mpg", "vob", "wmv",
    ];

    for film_path in film_paths.into_iter() {
        let extension = film_path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if valid_extensions.contains(&extension.as_str()) {
            match video_information(film_path.clone()) {
                Ok(video_information) => writer.serialize(video_information)?,
                Err(error) => {
                    tracing::warn!("Failed to parse {}: {}", film_path.display(), error);
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct VideoInformation {
    path: PathBuf,
    width: i32,
    height: i32,
    fps: f32,
    size_gib: f32,
    duration_minutes: f32,
    mib_per_minute: f32,
}

fn video_information(path: PathBuf) -> anyhow::Result<VideoInformation> {
    let path_str = path.to_str().context("invalid path")?;
    let output = Child::new(
        "ffprobe",
        &[
            "-hide_banner",
            "-v",
            "error",
            "-of",
            "json",
            "-show_entries",
            "stream=width,height,avg_frame_rate,codec_type:format=duration,size",
            path_str,
        ],
    )
    .capture_stdout()
    .run()
    .context("failed to execute ffprobe")?;

    let stdout = output.stdout()?;
    parse_video_information(path, &stdout)
        .with_context(|| format!("Failed to parse from:\n{}", stdout))
}

fn parse_video_information(
    path: PathBuf,
    command_output: &str,
) -> anyhow::Result<VideoInformation> {
    #[derive(Debug, Deserialize)]
    struct OutputJson {
        streams: Vec<OutputJsonStream>,
        format: OutputJsonFormat,
    }

    #[derive(Debug, Deserialize)]
    struct OutputJsonStream {
        codec_type: String,
        width: Option<i32>,
        height: Option<i32>,
        avg_frame_rate: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct OutputJsonFormat {
        duration: String,
        size: String,
    }

    let data: OutputJson = serde_json::from_str(command_output)?;
    let video_stream = data
        .streams
        .iter()
        .find(|s| s.codec_type == "video")
        .context("missing video stream")?;

    let width = video_stream.width.context("missing width")?;
    let height = video_stream.height.context("missing height")?;
    let fps_fraction = video_stream
        .avg_frame_rate
        .as_ref()
        .context("missing avg_frame_rate")?
        .split_once('/')
        .context("invalid avg_frame_rate")?;
    let fps = fps_fraction
        .0
        .parse::<f32>()
        .context("invalid avg_frame_rate")?
        / fps_fraction
            .1
            .parse::<f32>()
            .context("invalid avg_frame_rate")?;

    let size_bytes: f32 = data.format.size.parse().context("size")?;

    let duration_seconds: f32 = data.format.duration.parse().context("invalid duration")?;

    let duration_minutes = duration_seconds / 60.;

    Ok(VideoInformation {
        path,
        width,
        height,
        fps,
        size_gib: size_bytes / 1024. / 1024. / 1024.,
        duration_minutes,
        mib_per_minute: (size_bytes / 1024. / 1024.) / (duration_minutes),
    })
}
