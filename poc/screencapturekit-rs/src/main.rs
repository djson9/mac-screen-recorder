//! POC: 5-second screen recording using doom-fish/screencapturekit-rs
//!
//! Strategy:
//! 1. Enumerate shareable content (displays + windows).
//! 2. Build an SCContentFilter for the primary display.
//! 3. Configure an SCStream (1080p, BGRA, cursor on).
//! 4. Use SCRecordingOutput (macOS 15.0+) to write H.264/MP4 directly to disk
//!    — no manual AVAssetWriter or ffmpeg encoder needed.
//! 5. Also attach a frame-count handler to confirm frames are flowing.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use screencapturekit::prelude::*;
use screencapturekit::recording_output::{
    SCRecordingOutput, SCRecordingOutputCodec, SCRecordingOutputConfiguration,
    SCRecordingOutputFileType,
};

struct FrameCounter {
    count: Arc<AtomicUsize>,
}

impl SCStreamOutputTrait for FrameCounter {
    fn did_output_sample_buffer(&self, _sample: CMSampleBuffer, _of_type: SCStreamOutputType) {
        let n = self.count.fetch_add(1, Ordering::Relaxed);
        if n % 30 == 0 {
            println!("  frame {n}");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== screencapturekit-rs POC ===\n");

    // ---- 1. Enumerate shareable content ----
    println!("[1] Enumerating shareable content...");
    let content = SCShareableContent::get()?;
    let displays = content.displays();
    let windows = content.windows();
    println!(
        "    displays: {}, windows: {}",
        displays.len(),
        windows.len()
    );

    let display = displays.first().ok_or("no displays available")?;
    println!(
        "    primary display: {}x{}",
        display.width(),
        display.height()
    );

    // Show a few window titles to prove enumeration works
    let mut shown = 0usize;
    for w in windows.iter() {
        if let Some(title) = w.title() {
            if !title.is_empty() {
                println!("    window: \"{title}\"");
                shown += 1;
                if shown >= 5 {
                    break;
                }
            }
        }
    }

    // ---- 2. Build content filter ----
    println!("\n[2] Building content filter (full display)...");
    let filter = SCContentFilter::create()
        .with_display(display)
        .with_excluding_windows(&[])
        .build();

    // ---- 3. Stream configuration ----
    println!("[3] Configuring stream (1920x1080, BGRA, cursor on)...");
    let config = SCStreamConfiguration::new()
        .with_width(1920)
        .with_height(1080)
        .with_pixel_format(PixelFormat::BGRA)
        .with_shows_cursor(true);

    // ---- 4. Direct-to-file recording output (macOS 15.0+) ----
    let output_path = PathBuf::from("/tmp/sck_poc_recording.mp4");
    let _ = std::fs::remove_file(&output_path);

    println!(
        "[4] Configuring SCRecordingOutput -> {} (H264/MP4)",
        output_path.display()
    );
    let rec_config = SCRecordingOutputConfiguration::new()
        .with_output_url(&output_path)
        .with_video_codec(SCRecordingOutputCodec::H264)
        .with_output_file_type(SCRecordingOutputFileType::MP4);

    let recording_output =
        SCRecordingOutput::new(&rec_config).ok_or("failed to create SCRecordingOutput")?;

    // ---- 5. Build stream, attach handler + recording ----
    println!("[5] Creating SCStream and attaching outputs...");
    let mut stream = SCStream::new(&filter, &config);

    let count = Arc::new(AtomicUsize::new(0));
    stream.add_output_handler(
        FrameCounter {
            count: count.clone(),
        },
        SCStreamOutputType::Screen,
    );
    stream.add_recording_output(&recording_output)?;

    // ---- 6. Capture for 5 seconds ----
    println!("\n[6] Starting capture for ~5s.");
    println!(
        "    If no permission prompt has appeared before, macOS will surface one now\n    and the capture will yield zero frames until granted + restarted.\n"
    );
    stream.start_capture()?;
    std::thread::sleep(std::time::Duration::from_secs(5));
    stream.stop_capture()?;

    let frames = count.load(Ordering::Relaxed);
    let duration = recording_output.recorded_duration();
    let file_size = recording_output.recorded_file_size();
    println!("\n--- Results ---");
    println!("frames delivered:    {frames}");
    println!(
        "recording duration:  {} / {} (raw CMTime value/timescale)",
        duration.value, duration.timescale
    );
    println!("recording file size: {file_size} bytes (reported by SCRecordingOutput)");

    drop(stream);

    if let Ok(meta) = std::fs::metadata(&output_path) {
        println!("on-disk file size:   {} bytes", meta.len());
    } else {
        println!("on-disk file:        (not found)");
    }

    Ok(())
}
