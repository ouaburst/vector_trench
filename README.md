# Vector Trench

A compact retro vector-graphics trench-run arcade game with three playable builds: Python/Pygame, Rust for Apple Silicon macOS, and Rust for 64-bit Windows. Fly through a wireframe trench that turns, climbs, and descends. Destroy targets, dodge fighters and obstacles, and protect your hull. Survive the approach, then hit the reactor core to complete the mission.

## Play

**macOS (Apple Silicon):** Open `dist/macos/VectorTrench.app`, or run `dist/macos/vector_trench` from a terminal.

**Windows (64-bit):** Run `dist/windows/vector_trench.exe`.

**Python (macOS or Windows):** Install Python 3.10 or newer, then:

```bash
git clone https://github.com/ouaburst/vector_trench.git
cd vector_trench
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -r requirements.txt
python main.py
```

On Windows, activate the environment with `.venv\Scripts\activate` and use `python` in place of `python3`.

| Key | Action |
| --- | --- |
| 1 / 2 / 3 | Start the selected level from the opening screen |
| Up / Down, then Enter | Choose and start a level |
| W / S or Up / Down | Fly up / down |
| A / D or Left / Right | Fly left / right |
| Space | Fire continuously |
| M | Mute / unmute sound |
| R | Restart the current level |
| Tab | Return to level selection |
| Esc | Quit |

| Level | Forward speed | Obstacles |
| --- | ---: | --- |
| 1 — Training | 24 | Sparse |
| 2 — Assault | 31 | More frequent |
| 3 — Ace Run | 39 | Dense |

Keep a target or fighter centered in the reticle when firing. Targets score 100 points and fighters score 150. The final reactor core scores 1,000 points and ends the mission in victory. It sits low in the trench and is smaller than regular targets: descend and line up carefully once the final objective appears.

A missed target costs 12 hull points. Fighters steer toward you and launch visible laser bolts at your position; move to dodge them. The HUD warns of incoming fire and shows hits, misses, and damage. The mission fails if hull reaches zero or the reactor core passes by.

The trench bends left and right, climbs, and descends. Steer with the course: scraping a wall, floor, or ceiling costs 6 hull points, and hitting a red obstacle costs 20. The HUD shows the next turn and altitude change. Laser, hit, damage, warning, victory, and defeat sounds are bundled in `assets/`; both versions use the same original effects. Run `python3 tools/make_sounds.py` to regenerate them.

The game uses generated line art and original synthesized sounds. It requires no external art or audio downloads.

## Rust source and rebuilding

The Rust source is in `rust/` and uses Macroquad. From the project directory:

```bash
cargo test --manifest-path rust/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
```

The Mac build is `rust/target/release/vector_trench`. A Windows build on a Windows PC uses the same source and `cargo build --release`. To cross-compile a 64-bit Windows `.exe` from macOS, install the Rust target and a MinGW-w64 linker, then run:

```bash
rustup target add x86_64-pc-windows-gnu
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc cargo build --release --target x86_64-pc-windows-gnu --manifest-path rust/Cargo.toml
```

That produces `rust/target/x86_64-pc-windows-gnu/release/vector_trench.exe`.
