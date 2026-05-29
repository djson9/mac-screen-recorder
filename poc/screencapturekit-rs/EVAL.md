# `screencapturekit` (doom-fish) — Evaluation for a Mac Video-Recording CLI

Date: 2026-05-28 · macOS 15.6 (24G84), Apple Silicon · `cargo 1.94.1` ·
crate version evaluated: **`screencapturekit = "6.0.1"`** (latest), feature flag
`macos_15_0`.

## What I built

A 5-second screen recording to `/tmp/sck_poc_recording.mp4`, attaching both a
frame-counter callback and an `SCRecordingOutput` (H.264/MP4) to one
`SCStream`. Source: `src/main.rs` (~130 lines including comments/docs).

## Build experience

- `cargo build` (cold) completed in **20.3 s** with **zero errors and zero
  warnings**.
- 11 transitive crates total — `apple-cf`, `apple-metal`, `doom-fish-utils`,
  `futures-util`, `crossbeam-*`, `libc`, `pin-project-lite`. No `bindgen`,
  no `cocoa`, no `objc2` in the tree; the binding is hand-rolled FFI inside
  the parent crate.
- The build does compile a **Swift static library** under
  `target/.../build/{apple-cf,apple-metal,screencapturekit}-*/out/swift-build/` —
  i.e. there is a small Swift bridge shipped with the crates (visible in the
  build output as `swift-build/release/`). Xcode CLT must be installed.

### Runtime gotcha: `libswift_Concurrency.dylib @rpath` not resolved

The final binary's `otool -L` shows one bare `@rpath/libswift_Concurrency.dylib`
load entry **with no `LC_RPATH` set on the executable**. On macOS 15.6 that
library is not present at `/usr/lib/swift/`, so a default `cargo run` aborts
with `dyld: Library not loaded: @rpath/libswift_Concurrency.dylib`.

Workaround that worked here:

```bash
DYLD_FALLBACK_LIBRARY_PATH=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx \
  ./target/release/sck-poc
```

That produces noisy `objc[...]: Class _Tt... is implemented in both
/usr/lib/swift/libswift_Concurrency.dylib and ...swift-5.5/macosx/...` warnings
(because the dyld shared cache *does* have it), but the program runs
correctly. For a real CLI we'd need to either (a) link with
`-Wl,-rpath,/usr/lib/swift` via a `build.rs`, (b) bundle the dylib next to the
binary, or (c) file an issue upstream. This is a **real shipping blocker**
that I have not seen called out in the README.

## API surface — high-level, builder-pattern, idiomatic

This is **not** a 1:1 Apple binding you have to glue together yourself. It's
a deliberately Rust-idiomatic facade over `ScreenCaptureKit` with safe
ownership, `Send + Sync` output traits, and trait/closure handler equivalence.
For recording specifically, `SCRecordingOutput` + `add_recording_output`
means **you do not write any AVAssetWriter / VideoToolbox / ffmpeg encoder code**
for the standard case. Apple's framework encodes the file for you; the
binding is just plumbing.

Other modules also exposed and useful for a CLI:
`screenshot_manager`, `content_sharing_picker`, `async_api`, `metal`,
`recording_output`, `SCShareableContent::snapshot` (batched FFI).

## Code ergonomics — 9 / 10

The entire wiring of "find display, build filter, configure stream, attach
recording, run 5 s" is ~25 lines of business logic:

```rust
let content = SCShareableContent::get()?;
let display = content.displays().first().ok_or("no displays")?;

let filter = SCContentFilter::create()
    .with_display(display).with_excluding_windows(&[]).build();

let config = SCStreamConfiguration::new()
    .with_width(1920).with_height(1080)
    .with_pixel_format(PixelFormat::BGRA)
    .with_shows_cursor(true);

let rec_cfg = SCRecordingOutputConfiguration::new()
    .with_output_url(&PathBuf::from("/tmp/out.mp4"))
    .with_video_codec(SCRecordingOutputCodec::H264)
    .with_output_file_type(SCRecordingOutputFileType::MP4);
let recording = SCRecordingOutput::new(&rec_cfg).ok_or("rec init failed")?;

let mut stream = SCStream::new(&filter, &config);
stream.add_recording_output(&recording)?;
stream.start_capture()?;
std::thread::sleep(Duration::from_secs(5));
stream.stop_capture()?;
```

The one point off is the runtime-linking gotcha above.

## Real recording output

`ffprobe /tmp/sck_poc_recording.mp4`:

```
Duration: 00:00:05.00, bitrate: 2780 kb/s
Stream #0:0 Video: h264 (High) yuv420p 1920x1080, 56.91 fps, 600 tbn
```

- Container: **MP4**
- Codec: **H.264 High Profile**, `yuv420p`, 1920x1080
- Frame rate: ~57 fps (no `with_minimum_frame_interval` set — defaults to
  display vsync, capped at 60)
- File size: **1.72 MB** for 5 s
- Frames delivered to the Rust callback: **285** (the recording is written by
  Apple's pipeline, the callback is just for observation)
- `recording_output.recorded_duration()` returned `5 / 1` and
  `recorded_file_size()` matched `stat` exactly.

## Permission behavior

Standard ScreenCaptureKit TCC flow. The terminal already had Screen Recording
permission, so no prompt fired and capture started immediately. README is
explicit:

- No code-signing entitlement grants screen capture; it's purely TCC.
- A signed app needs `NSScreenCaptureUsageDescription` in `Info.plist` or
  the system terminates it when the prompt fires.
- For a CLI, the user grants the *parent terminal* (Terminal.app, iTerm,
  the IDE) and a restart of the calling process is required after grant.
- First-run with no grant: `SCShareableContent::get()` may return empty /
  zero frames will flow until the user grants and re-launches.

## Maintenance signals

- **Latest version**: 6.0.1, published 2026-05-20.
- **Open issues**: 0.
- **Stars**: 212.
- **Recent commits** (last 10 days): `Release v6.0.1`,
  `feat!: re-export CMSampleTimingInfo and CMClock from apple-cf`,
  `feat: derive Debug on every public struct that can take it`,
  `chore!: migrate to apple-cf 0.8 nested CGRect`,
  `feat!: drop CGImage and CMTime duplicates`. Project is **very active**
  but is making **breaking changes weekly** — major version 6 was cut in
  the same week as the eval. Six majors in the crate's life is a real
  churn signal.
- 24 runnable examples checked into the repo, including
  `19_ffmpeg_encoding.rs` (real-time H.264 via ffmpeg) and a full Metal
  viewer app — coverage is excellent.

## Verdict vs. `scap`

For comparison, `scap` is at `0.1.0-beta.1` (still beta), 611 stars,
47 open issues, last updated 2026-05-25. `scap` is *cross-platform*
(macOS+Windows+Linux/PipeWire) and Apache-2.0, but it does **not** wrap
`SCRecordingOutput`: with `scap` you get raw frames and have to encode
them yourself (its README points users at ffmpeg).

For a **macOS-only video-recording CLI** I would choose
**`screencapturekit` (doom-fish)** today, because:

1. `SCRecordingOutput` gives me an MP4 with Apple-managed H.264 encoding in
   ~25 lines and zero extra dependencies. With `scap` I'd have to bolt on
   ffmpeg or VideoToolbox myself.
2. The crate is genuinely safe-Rust ergonomic; the builder API is the
   nicest macOS-capture API I've seen in Rust.
3. Performance + feature coverage (audio, microphone, picker, screenshots,
   IOSurface, async) is comprehensive.

Caveats I'd want resolved before shipping:

- **Fix the `libswift_Concurrency.dylib @rpath` issue.** A user-installed
  CLI cannot rely on `DYLD_FALLBACK_LIBRARY_PATH`. Either the upstream
  build script needs to add `-Wl,-rpath,/usr/lib/swift` or the binding
  should avoid pulling in Swift Concurrency for the no-`async`-feature
  build. This is the only blocker.
- **Breaking-change cadence.** Six majors and weekly `feat!:` commits mean
  pin exactly and budget for upgrade churn.

If the CLI ever needs Windows or Linux support, switch to `scap` (or wrap
both). For macOS-only, `screencapturekit` wins.
