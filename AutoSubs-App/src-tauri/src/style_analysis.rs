use crate::audio_preprocess::run_ffmpeg;
use crate::transcript_types::{ColorModifier, Sample, Segment, Speaker, Transcript, WordTimestamp};
use dirs;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};
use tokio::process::Command as TokioCommand;

/// Extract frames from a video file using FFmpeg
/// Returns paths to extracted frame images
pub async fn extract_video_frames<R: Runtime>(
    app: AppHandle<R>,
    video_path: &str,
    interval_seconds: f64,
    max_frames: usize,
) -> Result<Vec<PathBuf>, String> {
    let cache_dir = app.path().app_cache_dir()
        .map_err(|e| format!("Failed to get cache directory: {}", e))?;

    let frames_dir = cache_dir.join("reference_frames");
    std::fs::create_dir_all(&frames_dir)
        .map_err(|e| format!("Failed to create frames directory: {}", e))?;

    // Use ffmpeg to extract frames at intervals
    let fps = 1.0 / interval_seconds;
    let output_pattern = frames_dir.join("frame_%04d.png").to_string_lossy().to_string();

    let args = vec![
        "-nostdin".to_string(),
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-i".to_string(),
        video_path.to_string(),
        "-vf".to_string(),
        format!("fps={}", fps),
        "-frames:v".to_string(),
        max_frames.to_string(),
        "-vsync".to_string(),
        "0".to_string(),
        output_pattern,
    ];

    let (success, _code, _stdout, stderr) = run_ffmpeg(&app, &args).await?;

    if !success {
        let stderr_str = String::from_utf8_lossy(&stderr);
        return Err(format!("FFmpeg failed: {}", stderr_str));
    }

    // Collect generated frame paths
    let mut frames = Vec::new();
    let entries = std::fs::read_dir(&frames_dir)
        .map_err(|e| format!("Failed to read frames directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("png") {
            frames.push(path);
        }
    }

    // Sort by filename
    frames.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    tracing::info!("Extracted {} frames from video", frames.len());
    Ok(frames)
}

/// Analyze a single frame for caption regions and colors
/// Focuses on the bottom portion of the frame where captions typically appear
fn analyze_frame_colors(frame_path: &PathBuf, caption_region_ratio: f32) -> Result<Vec<HashMap<String, f64>>, String> {
    use image::{DynamicImage, GenericImageView};
    use kmeans_colors::{get_kmeans, Kmeans};

    let img = image::open(frame_path)
        .map_err(|e| format!("Failed to load frame: {}", e))?;

    let (width, height) = img.dimensions();
    let caption_height = (height as f32 * caption_region_ratio) as u32;
    let caption_y_start = height - caption_height;

    // Crop to caption region (bottom portion)
    let caption_region = img.crop_imm(0, caption_y_start, width, caption_height);

    // Convert to RGB pixels
    let pixels: Vec<[f64; 3]> = caption_region
        .pixels()
        .map(|(_, _, pixel)| {
            let [r, g, b, _] = pixel.0;
            [r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0]
        })
        .collect();

    if pixels.is_empty() {
        return Ok(Vec::new());
    }

    // Use k-means clustering to find dominant colors
    // We want 3-5 clusters for fill, outline, shadow, background
    let k = 5;
    let kmeans = get_kmeans(k, pixels.clone(), 10, kmeans_colors::KmeansAlgorithm::Lloyd);

    let mut color_clusters = Vec::new();
    for cluster in kmeans.centroids {
        let mut cluster_info = HashMap::new();
        cluster_info.insert("r".to_string(), cluster[0]);
        cluster_info.insert("g".to_string(), cluster[1]);
        cluster_info.insert("b".to_string(), cluster[2]);
        // Count how many pixels belong to this cluster (approximate)
        let count = pixels.iter()
            .filter(|p| {
                let dr = p[0] - cluster[0];
                let dg = p[1] - cluster[1];
                let db = p[2] - cluster[2];
                (dr * dr + dg * dg + db * db).sqrt() < 0.15
            })
            .count();
        cluster_info.insert("count".to_string(), count as f64);
        color_clusters.push(cluster_info);
    }

    // Sort by count (dominance)
    color_clusters.sort_by(|a, b| {
        b.get("count").unwrap_or(&0.0).partial_cmp(a.get("count").unwrap_or(&0.0)).unwrap()
    });

    Ok(color_clusters)
}

/// Aggregate color analysis across multiple frames
/// Returns the most consistent colors found
fn aggregate_frame_colors(all_frame_colors: Vec<Vec<HashMap<String, f64>>>) -> HashMap<String, String> {
    use std::collections::HashMap;

    // Collect all colors with their frequencies
    let mut color_accumulator: HashMap<(u8, u8, u8), usize> = HashMap::new();

    for frame_colors in all_frame_colors {
        for cluster in frame_colors {
            let r = (cluster.get("r").unwrap_or(&0.0) * 255.0) as u8;
            let g = (cluster.get("g").unwrap_or(&0.0) * 255.0) as u8;
            let b = (cluster.get("b").unwrap_or(&0.0) * 255.0) as u8;
            let count = cluster.get("count").unwrap_or(&0.0) as usize;

            *color_accumulator.entry((r, g, b)).or_insert(0) += count;
        }
    }

    // Sort by frequency
    let mut sorted_colors: Vec<_> = color_accumulator.into_iter().collect();
    sorted_colors.sort_by(|a, b| b.1.cmp(&a.1));

    // Convert top colors to hex
    let mut result = HashMap::new();
    for (i, ((r, g, b), _)) in sorted_colors.into_iter().take(5).enumerate() {
        let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
        match i {
            0 => { result.insert("fill_color".to_string(), hex); }
            1 => { result.insert("outline_color".to_string(), hex); }
            2 => { result.insert("shadow_color".to_string(), hex); }
            3 => { result.insert("background_color".to_string(), hex); }
            _ => {}
        }
    }

    result
}

/// Analyze reference video and extract caption style parameters
pub async fn analyze_reference_video_internal<R: Runtime>(
    app: AppHandle<R>,
    video_path: &str,
) -> Result<crate::transcription_api::ReferenceVideoAnalysisResult, String> {
    tracing::info!("Starting reference video analysis for: {}", video_path);

    // Extract frames - every 2 seconds, max 15 frames
    let frames = extract_video_frames(app.clone(), video_path, 2.0, 15).await?;

    if frames.is_empty() {
        return Err("No frames extracted from video".to_string());
    }

    // Analyze each frame for caption region colors
    let mut all_frame_colors = Vec::new();
    for frame_path in &frames {
        match analyze_frame_colors(frame_path, 0.25) { // bottom 25% of frame
            Ok(colors) => all_frame_colors.push(colors),
            Err(e) => tracing::warn!("Failed to analyze frame {}: {}", frame_path.display(), e),
        }
    }

    // Aggregate results
    let aggregated = aggregate_frame_colors(all_frame_colors);

    // Create result
    let detected_frames = frames.len();

    // Clean up temporary frames
    for frame_path in frames {
        let _ = std::fs::remove_file(frame_path);
    }

    Ok(crate::transcription_api::ReferenceVideoAnalysisResult {
        fill_color: aggregated.get("fill_color").cloned(),
        outline_color: aggregated.get("outline_color").cloned(),
        shadow_color: aggregated.get("shadow_color").cloned(),
        position: Some([0.5, 0.12, 0.0]),
        text_size: Some(0.07),
        speaker_colors: None, // Would need diarization + timestamp alignment
        detected_frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_conversion() {
        let r = 255u8;
        let g = 215u8;
        let b = 0u8;
        let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
        assert_eq!(hex, "#FFD700");
    }
}