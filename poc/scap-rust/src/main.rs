// scap PoC: enumerate targets, capture ~5s of BGRA frames at 30fps,
// pipe to ffmpeg to produce an H.264 mp4.

use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::{Frame, FrameType, VideoFrame},
    Target,
};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

fn main() {
    if !scap::is_supported() {
        eprintln!("Platform not supported by scap");
        std::process::exit(1);
    }

    if !scap::has_permission() {
        eprintln!("Screen Recording permission not granted; requesting...");
        if !scap::request_permission() {
            eprintln!("Permission denied. Grant Screen Recording for the terminal in System Settings > Privacy & Security > Screen Recording, then re-run.");
            std::process::exit(2);
        }
    }

    // Enumerate targets
    let targets = scap::get_all_targets();
    println!("Found {} targets:", targets.len());
    for t in &targets {
        match t {
            Target::Display(d) => println!("  [display] id={} title={:?}", d.id, d.title),
            Target::Window(w) => println!("  [window]  id={} title={:?}", w.id, w.title),
        }
    }

    // Pick the main display
    let display_target = targets.iter().find_map(|t| match t {
        Target::Display(_) => Some(t.clone()),
        _ => None,
    });

    let fps: u32 = 30;
    let duration_secs: u64 = 5;

    let options = Options {
        fps,
        target: display_target,
        show_cursor: true,
        show_highlight: false,
        excluded_targets: None,
        output_type: FrameType::BGRAFrame,
        output_resolution: Resolution::_720p,
        crop_area: None,
        captures_audio: false,
        exclude_current_process_audio: false,
    };

    let mut capturer = match Capturer::build(options) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Capturer::build failed: {e}");
            std::process::exit(3);
        }
    };

    capturer.start_capture();
    println!("Capture started; pulling first frame to learn dimensions...");

    // Pull the first video frame to learn dimensions.
    let (width, height, first_data) = loop {
        match capturer.get_next_frame() {
            Ok(Frame::Video(v)) => match v {
                VideoFrame::BGRA(f) => break (f.width, f.height, f.data),
                _other => {
                    eprintln!("Unexpected video frame variant; ignoring");
                    continue;
                }
            },
            Ok(Frame::Audio(_)) => continue,
            Err(e) => {
                eprintln!("recv error before first frame: {e}");
                capturer.stop_capture();
                std::process::exit(4);
            }
        }
    };

    println!(
        "First frame: {}x{} ({} bytes BGRA)",
        width,
        height,
        first_data.len()
    );

    // Spawn ffmpeg, feed it raw BGRA frames on stdin, output H.264 mp4.
    let output_path = "out.mp4";
    let size_arg = format!("{}x{}", width, height);
    let fps_arg = format!("{}", fps);

    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgra",
            "-s",
            &size_arg,
            "-r",
            &fps_arg,
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-pix_fmt",
            "yuv420p",
            "-r",
            &fps_arg,
            output_path,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn ffmpeg. Is it installed?");

    let mut ffstdin = ffmpeg.stdin.take().expect("ffmpeg stdin");
    ffstdin.write_all(&first_data).expect("ffmpeg write 0");
    let mut frame_count: u64 = 1;

    let start = Instant::now();
    let stop_at = std::time::Duration::from_secs(duration_secs);
    while start.elapsed() < stop_at {
        match capturer.get_next_frame() {
            Ok(Frame::Video(VideoFrame::BGRA(f))) => {
                if f.width != width || f.height != height {
                    eprintln!(
                        "Frame size changed mid-capture: {}x{} -> {}x{}, skipping",
                        width, height, f.width, f.height
                    );
                    continue;
                }
                if let Err(e) = ffstdin.write_all(&f.data) {
                    eprintln!("ffmpeg write failed: {e}");
                    break;
                }
                frame_count += 1;
            }
            Ok(Frame::Video(_other)) => {
                // Other variant arrived; ignore.
            }
            Ok(Frame::Audio(_)) => continue,
            Err(e) => {
                eprintln!("recv error mid-capture: {e}");
                break;
            }
        }
    }

    println!(
        "Stopping capture after {} frames in {:.2}s ({:.1} fps effective)",
        frame_count,
        start.elapsed().as_secs_f64(),
        frame_count as f64 / start.elapsed().as_secs_f64()
    );

    capturer.stop_capture();

    drop(ffstdin); // close pipe so ffmpeg finalizes
    let status = ffmpeg.wait().expect("ffmpeg wait");
    println!("ffmpeg exited: {}", status);
    println!("Wrote: {}", output_path);
}
