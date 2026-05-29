# Azayaka Evaluation

Repo: https://github.com/Mnpn/Azayaka (cloned at `Azayaka/`, latest commit `9fb0ac9`, 2025-08-26).

## Repo type
**Pure menu-bar SwiftUI app.** It is *not* a CLI tool and has no hybrid mode.

- `@main struct Azayaka: App` with `Settings { Preferences() }` scene (`AppDelegate.swift:17`).
- All recording is triggered by `@objc func prepRecord(_ sender: NSMenuItem)` (`Recording.swift:13`) — requires an `NSMenuItem` with `identifier.rawValue` of `"window" | "display" | "audio"`. No code path bypasses the menu.
- No `CommandLine.arguments`, `ProcessInfo.arguments`, or argv handling anywhere in the source (`grep` over all `.swift` files returns zero hits).
- The README only describes the GUI ("menu bar-residing screen and system audio recorder") — no CLI flags advertised.

## Build experience
Clean and easy.

- Toolchain: Swift 6.2.4 / Xcode 26.3 on macOS 15 SDK target.
- No `Package.swift`; uses `Azayaka.xcodeproj` with one external SPM dependency (`sindresorhus/KeyboardShortcuts` 2.2.3).
- `xcodebuild -project Azayaka.xcodeproj -scheme Azayaka -configuration Release -derivedDataPath ./build CODE_SIGNING_ALLOWED=NO build` → **BUILD SUCCEEDED** in ~1 min, universal binary at `build/Build/Products/Release/Azayaka.app/Contents/MacOS/Azayaka` (3.1 MB).

## CLI / automation surface
**None.** Running `./Azayaka --record-display` for 5s produced no file on Desktop; flags are silently ignored and the process just tries to install a menu bar item (which it cannot without LaunchServices registration / TCC permission).

Automation hooks that *do* exist, but are all GUI-bound:
- Global hotkeys via `KeyboardShortcuts` package (`Shortcuts.swift`) — could in principle trigger record/stop, but require the app to be registered and the user to bind the shortcut in Preferences first.
- Settings live in `UserDefaults` (`com.apple.screencapture` location, file name template with `%t`, encoder, fps, etc.) — scriptable via `defaults write dev.mnpn.Azayaka …`, but still need a human/GUI to click the menu to start.

## Did it record?
**No.** Headless invocation produced zero output. Expected — there is no code path to start a capture without an `NSMenuItem` click.

## Reusable Swift / ScreenCaptureKit code
Quality is **good and forkable** if the goal is to write our own CLI on top of it. Highlights:

- `Recording.swift:70 record(audioOnly:filter:)` — clean `SCStream` setup with `SCStreamConfiguration` (HDR preset, frame interval, audio rate/channels, mic capture on macOS 15).
- `Processing.swift:13 initVideo(conf:)` — dual-path writer: new `SCRecordingOutput` on macOS 15+, falls back to `AVAssetWriter`.
- `ClassicProcessing.swift` — full `AVAssetWriter` pipeline with H.264/HEVC, bitrate heuristic (`width*height*fpsMultiplier*encoderMultiplier*quality`), separate video/audio/mic inputs, and `CMSampleBuffer` plumbing in `stream(_:didOutputSampleBuffer:of:)`.
- `Types.swift` enums (`StreamType`, `VideoFormat`, `Encoder`, `AudioFormat`) are tidy.

Caveats for forking:
- Everything is hung off `AppDelegate` extensions and reads from `UserDefaults` directly — would need to extract into a plain `Recorder` class taking a config struct.
- ~1837 lines total; the actual recording pipeline (`Recording.swift` + `Processing.swift` + `ClassicProcessing.swift`) is only ~540 lines and is the only part worth taking. The other ~1300 lines are menus, preferences UI, localizations, countdown UI, notifications, updates.
- License: not checked here — verify before copying.

## Maintenance signals
- 47 commits since 2024-01-01, only **2 commits in 2025**, latest **2025-08-26** (~9 months stale). Single maintainer (Martin Persson). It's alive but slow.
- Targets macOS 13+, already handles macOS 15 `SCRecordingOutput` API. Code is current.

## Verdict
**Pass as a CLI. Possibly worth cannibalizing as a reference.**

- As a turnkey CLI recorder: **no** — zero argv support, GUI-locked, requires TCC + menu bar.
- As a fork target: only marginal value. The ScreenCaptureKit + AVAssetWriter pipeline is competent and dual-paths macOS 13/14 vs 15, but it's also ~500 lines tightly coupled to `AppDelegate`/`UserDefaults`. You'd spend more time decoupling than writing the SCK glue from scratch against Apple's sample code.
- Better options for a real CLI: write a thin Swift Package executable directly against `ScreenCaptureKit` (Apple's docs are sufficient), or evaluate purpose-built CLIs like `nonstrict/QuickRecorder` CLI mode, `lwouis/alt-tab-macos`-style tools, or `screencapture(1)`'s `-v` flag for short clips.

**Recommendation: pass, keep `Recording.swift`/`Processing.swift` bookmarked as reference for SCK + AVAssetWriter dual-path patterns.**
