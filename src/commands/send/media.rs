use anyhow::Result;
use std::path::Path;
use tdlib_rs::{enums, types};

#[derive(Clone, Copy)]
pub enum MediaType {
    Photo,
    Video,
    Audio,
    Document,
}

impl MediaType {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaType::Photo => "photo",
            MediaType::Video => "video",
            MediaType::Audio => "audio",
            MediaType::Document => "document",
        }
    }
}

fn detect_media_type(path: &str) -> Result<MediaType> {
    let path = Path::new(path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .ok_or_else(|| anyhow::anyhow!("No file extension found for {}", path.display()))?;

    let media_type = match extension.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "heic" => MediaType::Photo,
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "flv" | "m4v" => MediaType::Video,
        "mp3" | "wav" | "ogg" | "m4a" | "flac" | "aac" | "opus" => MediaType::Audio,
        _ => MediaType::Document,
    };

    Ok(media_type)
}

fn get_video_metadata(path: &str) -> Result<(i32, i32, i32)> {
    match ffprobe::ffprobe(path) {
        Ok(info) => {
            let video_stream = info
                .streams
                .iter()
                .find(|stream| stream.codec_type == Some("video".to_string()));

            let (width, height) = video_stream
                .map(|stream| {
                    (
                        stream.width.unwrap_or(0) as i32,
                        stream.height.unwrap_or(0) as i32,
                    )
                })
                .unwrap_or((0, 0));

            let duration = info
                .format
                .duration
                .and_then(|duration| duration.parse::<f64>().ok())
                .map(|duration| duration as i32)
                .or_else(|| {
                    video_stream
                        .and_then(|stream| stream.duration.as_ref())
                        .and_then(|duration| duration.parse::<f64>().ok())
                        .map(|duration| duration as i32)
                })
                .unwrap_or(0);

            Ok((width, height, duration))
        }
        Err(error) => {
            eprintln!("Warning: Failed to extract video metadata: {}", error);
            Ok((0, 0, 0))
        }
    }
}

pub fn build_media_content(
    path: &str,
    caption: Option<types::FormattedText>,
) -> Result<(enums::InputMessageContent, MediaType)> {
    if !Path::new(path).exists() {
        anyhow::bail!("File not found: {}", path);
    }

    let media_type = detect_media_type(path)?;
    let input_file = enums::InputFile::Local(types::InputFileLocal {
        path: path.to_string(),
    });

    let content = match media_type {
        MediaType::Photo => {
            enums::InputMessageContent::InputMessagePhoto(types::InputMessagePhoto {
                photo: input_file,
                thumbnail: None,
                added_sticker_file_ids: vec![],
                width: 0,
                height: 0,
                caption,
                show_caption_above_media: false,
                self_destruct_type: None,
                has_spoiler: false,
            })
        }
        MediaType::Video => {
            let (width, height, duration) = get_video_metadata(path)?;
            enums::InputMessageContent::InputMessageVideo(types::InputMessageVideo {
                video: input_file,
                thumbnail: None,
                added_sticker_file_ids: vec![],
                duration,
                width,
                height,
                supports_streaming: true,
                caption,
                show_caption_above_media: false,
                self_destruct_type: None,
                has_spoiler: false,
                cover: None,
                start_timestamp: 0,
            })
        }
        MediaType::Audio => {
            enums::InputMessageContent::InputMessageAudio(types::InputMessageAudio {
                audio: input_file,
                album_cover_thumbnail: None,
                duration: 0,
                title: String::new(),
                performer: String::new(),
                caption,
            })
        }
        MediaType::Document => {
            enums::InputMessageContent::InputMessageDocument(types::InputMessageDocument {
                document: input_file,
                thumbnail: None,
                disable_content_type_detection: false,
                caption,
            })
        }
    };

    Ok((content, media_type))
}
