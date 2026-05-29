# scap (Rust) PoC Evaluation

Crate: `scap = "0.1.0-beta.1"` (latest on crates.io, published 2025-08-04). Repo: CapSoftware/scap.

## Build experience

- `cargo init` + adding `scap` to Cargo.toml: zero friction.
- Clean release build on macOS arm64: **23.4s**, 56 crates pulled. No external system deps required to compile (heavy lifting is via the `cidre` crate, which wraps ScreenCaptureKit/AVFoundation — no Xcode framework hunting on my end). `ffmpeg` is only needed at runtime because scap does not encode.
- Zero compile warnings/errors with the README example shape. MSRV is 1.85.

## API ergonomics: 7/10

It's a thin, idiomatic wrapper. The happy path is short and obvious; rough edges are around frame variants and lack of an encoded-output story.

```rust
let options = Options {
    fps: 30,
    target: display_target,           // Option<Target>
    output_type: FrameType::BGRAFrame,
    output_resolution: Resolution::_720p,
    ..Default::default()
};
let mut cap = Capturer::build(options)?;
cap.start_capture();
loop {
    match cap.get_next_frame()? {
        Frame::Video(VideoFrame::BGRA(f)) => ffmpeg_stdin.write_all(&f.data)?,
        Frame::Video(_) | Frame::Audio(_) => continue,
    }
}
cap.stop_capture();
```

Pain points:
- `get_next_frame()` blocks; no async/iterator/stream wrapper. You hand-roll the recv loop.
- `VideoFrame` is a 7-variant enum (YUV/RGB/RGBx/XBGR/BGRx/BGR0/BGRA) and you must match exhaustively even though you asked for one `FrameType`. In practice you only get the requested variant, but the type system doesn't reflect that.
- No way to query the chosen output dimensions before the first frame arrives — you must pull a frame to learn `width`/`height`.
- `Options` requires explicitly listing all fields if you want to be safe (`..Default::default()` works but hides booleans like `captures_audio`).
- `start_capture`/`stop_capture` return `()`, not `Result`. Errors surface only as `RecvError` on the channel.

## Encoded video? No.

scap is **frame-pump only** on macOS. You pick BGRA/RGB/YUV/etc; you do not get H.264 out. To produce a file you need an external encoder. Two practical workflows:

1. **Pipe BGRA to `ffmpeg` over stdin** (what this PoC does). Works, adds a process dependency, and burns CPU re-encoding raw frames. Good for a CLI shipping with ffmpeg bundled.
2. **Use VideoToolbox directly** (`cidre` already a transitive dep) to feed `CVPixelBuffer`s into an `AVAssetWriter` H.264/HEVC session. Much faster (zero-copy from ScreenCaptureKit), but you lose scap's abstraction and you're writing platform-specific code anyway — at which point you'd reconsider whether scap is the right base.

scap also exposes `Frame::Audio` (mac/win), but does not mux it with video.

## Output (from PoC)

Captured 5s of the primary display at 30fps, requested 720p.

- File: `out.mp4`, 200 KB
- Codec: H.264 High@L3.2, yuv420p
- Resolution: 1280x830 (scap fit 1280-wide to display aspect; height isn't exactly 720)
- Frame rate: 30 fps avg, 150 frames, duration 5.00s
- Effective capture: 29.9 fps measured — no dropped frames
- Bitrate: ~323 kbit/s (libx264 CRF 23, near-static screen)

Quality looks correct; effective fps matches requested.

## Permission behavior

- `scap::has_permission()` returns the TCC state for Screen Recording on the *parent process* (the terminal).
- `scap::request_permission()` triggers the standard macOS prompt; granting requires re-launching the requesting app. On a machine where the terminal is already approved, this PoC ran without any prompt or extra UX.
- No microphone/camera prompt was triggered (we set `captures_audio: false`).

## Maintenance signals

- Latest release: `0.1.0-beta.1`, **2025-08-04**.
- Last commit on `main`: **2025-08-05** ("fix windows build"). ~9 months stale as of today (2026-05-28).
- Active period through mid-2025 (audio capture PR landed). Owned by Cap (CapSoftware), the company behind the Cap screen recorder, so it's load-bearing for a real product — but the public crate has not seen new releases in a while.
- 7 listed contributors; depends on `cidre` (mac), `windows-capture` (win), `pipewire` (linux).

## Verdict

**Decent foundation for a Mac-only recorder PoC; not the obvious choice for a polished CLI.**

Pros: trivial setup, gives you ScreenCaptureKit without writing FFI, handles target enumeration and permission probing, cross-platform if you care.

Cons: no encoding (you'll ship/spawn ffmpeg or drop down to VideoToolbox anyway), blocking pull-based frame API, dimensions-unknown-until-first-frame, no audio muxing, and the project is in beta with the last commit 9 months ago. If the goal is a custom CLI with H.264/HEVC + audio + chunked recording, the work *not* covered by scap (encoding, muxing, audio sync, hardware acceleration) is the bulk of the project — at which point a thinner wrapper directly on `cidre`/ScreenCaptureKit + `AVAssetWriter` may actually be less total code than scap + ffmpeg piping.

Recommendation: fine for a 1-week PoC; for production, plan on either contributing an encode path upstream or going straight to ScreenCaptureKit + VideoToolbox.
