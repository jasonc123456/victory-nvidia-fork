# victory-nvidia-fork: VIctory on NVIDIA

A fork of the VIctory-7900XTX fork (avatarmanz/VIctory-7900XTX, itself from fxceless/victory-fork),
with changes for NVIDIA cards. It holds no title files: you supply your own.

Upstream's work is merged in up to avatarmanz 9c23e8d (2026-11-26). It includes the processor-side
speedups (`VICTORY_SYNC_IN_PLACE`, `VICTORY_SPANS_QUICK`, `VICTORY_METER_THREAD`, `VICTORY_SEND_OWN`,
`VICTORY_SHOW_NEW_LATER`, all on) and switches still off by default, to try through `MORE=`:
`VICTORY_UP_QUEUE=1`, `VICTORY_JOIN_BARRIERS=1`, `VICTORY_LANE_CORE=1`, `VICTORY_PORT_CONSTANTS=1`.

## Changes on top of the 7900XTX fork

- **NVIDIA shader fixes (32-lane waves)**, from fxceless's pull requests:
  - Console wave64 compute programs run on 32-lane cards in a paired form: each host lane runs
    console lanes n and n + 32 (#854, Pilote Production, `VICTORY_WAVE_PAIRS=0` turns it off).
  - Mesh programs whose outputs pass D3D12's 32 KiB are linked with their pixel program
    (#854, `VICTORY_MESH_LINK=0` turns it off).
  - Four compute programs the driver refused run in the wave-wide form, and the ray-tracing
    structure builder's three scan programs read the host wave's own first lane, chosen per
    program (#823; `VICTORY_WAVE_WIDE_ONLY` and `VICTORY_FIRSTLANE_OWN_ONLY` change the lists).
  - The thread-group info register on 32-lane waves (#799), `ds_bpermute` across the two halves
    (#615), LDS flags raised inside a branch (#592), a choice between two immediate samplers and
    mesh vertex indices without their edge flag (#1112), and #1006.

  They were collected for NVIDIA by blacksi (mauri9911) and ported here onto avatarmanz's newer
  shader translator. Where both had changed the same code, this tree's version is kept:
  descriptor reads as data, ordering by primitive (POPS), and spilled-register tracking.
- **Series S display**: an upscaler stand-in and the interface layer, from fxceless PR 814.
  Without it, Series S mode shows a black screen with sound.
- **Camera jitter** is taken out of that upscale.
- **Ray tracing on NVIDIA**: the structure builder's three programs that need the host wave's own
  first lane get it by default (#823 above). Without it, 3 of the builder's 53 programs do not
  translate, and ray tracing stays off. The launcher used to set `VICTORY_FIRSTLANE_OWN=1`, which
  applies it to every program; it no longer does.
- **Controllers** get a new reading every 8 ms, not only when they change (`VICTORY_PAD_REPEAT`).
- **Descriptor slots are given back** as soon as the card has run a submission
  (`VICTORY_EARLY_TABLES`). NVIDIA allows only 1,000,000 views, which leaves 262,144 slots for
  per-draw tables. Holding them 64 submissions longer froze the picture: draws were dropped and
  the image grew brighter.
- **More logging** for upscales and controller input.
- **Launcher script** (`scripts/nvidia/start.ps1`): builds this checkout, sets the options, keeps
  each run's log and shows crash reports. **`Collect logs.cmd`** zips the logs for sharing.

## Setup

Needs Windows 10 or 11 and an NVIDIA RTX 30 series card or newer with a current driver.
Skip steps 1 and 2 on a PC that has built VIctory before.

1. **Rust**: install it from https://rustup.rs, or in a terminal: `winget install Rustlang.Rustup`.
   The exact compiler version is pinned in `rust-toolchain.toml` and downloaded on the first build.
2. **Visual Studio C++ build tools**: install "Build Tools for Visual Studio" with the workload
   "Desktop development with C++", or in a terminal:
   `winget install Microsoft.VisualStudio.2022.BuildTools --override "--passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
3. **This repository**: `git clone https://github.com/jasonc123456/victory-nvidia-fork.git`
4. **Your start file**: create it as described below.

## Creating your start file

The start file is not part of the repository; git ignores it. It holds no path of its own, so the
same file works on any Windows PC with an NVIDIA RTX 30 series card or newer: copy it along with
the repository.

1. In the repository's top folder, create a text file named **`Start VIctory NVIDIA.cmd`**.
   In Notepad: *File > Save as*, set *Save as type* to *All files*, and type the name with
   `.cmd` at the end.
2. Paste this into it:

   ```bat
   @echo off
   setlocal
   rem VIctory on an NVIDIA card (RTX 30 series or newer). Change the settings, save, double-click.
   rem 1 = on, 0 = off.

   rem The title's main executable. Empty: a file picker asks on the first start on each PC and
   rem the choice is remembered. A relative path is taken from this folder.
   set "GAME="

   set RAYTRACING=1
   set SEPARATE_PADS=1
   set SAFE=1
   set SERIES_X=0
   rem auto: on for cards with 8 GB or less.
   set LOW_VRAM=auto
   set RELEASE=0
   set INPUT_TRACE=0
   rem Further settings, space-separated, e.g. set "MORE=VICTORY_TEXTURE_ROOM_PERCENT=85"
   set "MORE="

   if not exist "%~dp0scripts\nvidia\start.ps1" (
       echo This file belongs in the top folder of victory-nvidia-fork, next to NVIDIA.md.
       pause
       exit /b 1
   )
   set "FLAGS="
   if "%RAYTRACING%"=="1"    set "FLAGS=%FLAGS% -RayTracing"
   if "%SEPARATE_PADS%"=="1" set "FLAGS=%FLAGS% -SeparatePads"
   if "%SAFE%"=="1"          set "FLAGS=%FLAGS% -Safe"
   if "%SERIES_X%"=="1"      set "FLAGS=%FLAGS% -SeriesX"
   if "%LOW_VRAM%"=="1"      set "FLAGS=%FLAGS% -LowVram"
   if "%LOW_VRAM%"=="0"      set "FLAGS=%FLAGS% -FullVram"
   if "%RELEASE%"=="1"       set "FLAGS=%FLAGS% -Release"
   if "%INPUT_TRACE%"=="1"   set "FLAGS=%FLAGS% -InputTrace"
   if defined MORE set FLAGS=%FLAGS% -Settings "%MORE%"
   set "VICTORY_GAME=%GAME%"
   powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\nvidia\start.ps1" %FLAGS% %*
   pause
   ```

3. Set the options (below), save, and double-click the file. On the first start on a PC, pick
   the title's main executable in the window that opens. To pick a different one later, start
   the file from a terminal with `-PickGame`.

On every start, the launcher checks:

- **The card**: it names the GPU, its memory and driver, and warns if the card is older than the
  RTX 30 series or no NVIDIA card is found.
- **The build tools**: if Rust or the Visual Studio C++ tools are missing, it stops and says how
  to install them (steps 1 and 2 above).

| Setting         | Suggested | What it does                                                       |
|-----------------|-----------|--------------------------------------------------------------------|
| `GAME`          | empty     | The title's main executable. Empty: picked once per PC             |
| `RAYTRACING`    | 1         | Ray-traced light (lit shadows)                                     |
| `SEPARATE_PADS` | 1         | Each controller is its own device                                  |
| `SAFE`          | 1         | Without the fork's newest CPU shortcuts. Steadier, and no welcome-screen stepping. Try 0 on a strong CPU |
| `SERIES_X`      | 0         | Series X mode instead of Series S. Needs more video memory         |
| `LOW_VRAM`      | auto      | Less video memory for textures and copies. auto: on for 8 GB cards or smaller |
| `RELEASE`       | 0         | Release build instead of the diagnostics build                     |
| `INPUT_TRACE`   | 0         | Log every controller reading                                       |
| `MORE`          | empty     | Further settings, e.g. `VICTORY_TEXTURE_ROOM_PERCENT=85`           |

Flags typed after the file's name in a terminal are added to the ones set in the file:

    & ".\Start VIctory NVIDIA.cmd" -SeriesX

Every start runs `cargo build`, so changes you pull are built. When nothing has changed, the
build takes seconds. The first build takes several minutes. The first run also translates every
shader, so loading is slow, and some things are missing or look wrong until their shaders are
ready.

## Logs

| What       | Where                                                    |
|------------|----------------------------------------------------------|
| Run logs   | `%LOCALAPPDATA%\VIctory-Fork\logs\run-<time>-<flags>.log` (the last 10, UTF-16) |
| Crashes    | `%LOCALAPPDATA%\VIctory-Fork\data-xtx\<title id>\crashes.log` |
| Saves      | `%LOCALAPPDATA%\VIctory-Fork\data-xtx`                   |
| Cache      | `%LOCALAPPDATA%\VIctory-Fork\cache-xtx`                  |
| Build      | `%LOCALAPPDATA%\VIctory-Fork\build-xtx`                  |

`Collect logs.cmd` puts the last 5 run logs, the crash log and the GPU, driver and drive details
into `victory-logs.zip` on the desktop. Nothing is sent anywhere.

The window title shows fps, milliseconds per frame and draws per frame. In the log, these lines
show where a frame's time goes:

- `<n> fps, <n> ms/frame (<n> preparing ... <n> on the GPU ...)`: CPU time against GPU time.
- `dynamic slots: <n> tables asked, <n> refused`: should show 0 refused.
- `video memory as the system counts it`: the card's budget and use.

## Results

| Machine                                      | Settings                                        | Result |
|----------------------------------------------|-------------------------------------------------|--------|
| RTX 4060 Laptop 8 GB, i7-13650HX             | ray tracing, separate pads, safe, low VRAM      | 4.2-4.7 fps in the first playable area, complete picture. GPU 110-165 ms and CPU 61-71 ms per frame; video memory full |
| RTX 5070 12 GB, i7-14700K, 32 GB             | ray tracing, separate pads, safe, low VRAM auto (off) | Before upstream b0cfe2b: 4.5-5.7 fps in the world (dips to 2.2). 130-160 ms of each frame preparing on the processor side, 40-50 ms copying back; 11.2 of 12 GB video memory plus 8.1 GB shared, RAM at 96% |
| RTX 5070 12 GB, i7-14700K, 32 GB             | same, with upstream b0cfe2b                     | About 7 fps on average in an early indoor area; visual faults remain |
| RTX 5070 12 GB, i7-14700K, 32 GB             | same, at one spot of that area: without / with the shader fixes | Without: 6.3 fps, 102 ms preparing, 31 ms GPU, 3,237 draws. With: 6.0 fps, 87 ms preparing, 59 ms GPU, 4,581 draws, no pipeline refused, the structure builder on with all 53 pipelines. The same within run-to-run noise (6-7 fps either way); visual faults unchanged |

With the shader fixes, the first start after an update rebuilds every pipeline while it plays
(hundreds at first, 1.5-4 fps for a minute or two); the second start loads them from the cache.
Still to compare: `LOW_VRAM=1` against `auto` on a 12 GB card.

### Shader pack on an RTX 5070 (32-lane waves)

`victory translate --validate --pipelines` over the title's shader pack and its ray-tracing
structure builder, with `VICTORY_HOST_WAVE=32` and `VICTORY_VIEW_LIMIT=999999` as a run on this
card reports them (without them the command translates for 64-lane waves, and NVIDIA's driver
refuses those). 26,136 programs; translated, validated by the Windows SDK's `dxil.dll`, pipeline
made by the driver:

| Programs                  | Before the shader fixes (487d05a)      | With them                              |
|---------------------------|----------------------------------------|----------------------------------------|
| Vertex and pixel (14,247) | 14,247 / 14,220 / 14,247               | 14,247 / **14,247** / 14,247           |
| Compute (2,734)           | 2,726 / 2,726 / **2,697** (29 refused) | 2,729 / 2,729 / **2,729** (0 refused)  |
| Mesh (8,765)              | 8,765 / 8,673 / 8,765                  | 8,765 / 8,694 / 8,765                  |

- The 29 refused compute programs included particle visibility and emitters, hair shadow masks,
  screen-space reflection tiling and an alpha transmission filter. All now get pipelines.
- The structure builder's 3 programs that did not translate without `VICTORY_FIRSTLANE_OWN=1`
  now translate, so all of its programs have pipelines.
- 27 vertex and pixel programs and 21 mesh programs that the validator refused (signature rows
  packed with mixed interpolation, a texel coordinate out of bounds) now validate.
- Left: 71 mesh programs over D3D12's 32 KiB of outputs alone (linked with their pixel program
  during a run, #854), 5 compute programs that read a descriptor only as data, and 195 voxel
  geometry programs the title does not use.
- No program that passed before fails now; 83 more pass.

## Known faults

- Green and red stripes, or a pixelated look, on some characters' arms.
- The welcome screen steps back and forth without `SAFE`.
- Grainy speckle on hair and walls (ray-tracing noise), and bright stair-step blocks near windows.
- Before the shader fixes, the driver refused compute pipelines with E_INVALIDARG (four in a run,
  29 over the whole pack), and each dropped its work. With them, every compute program gets a
  pipeline, and a run refuses none.
- Brief flashes of wrong-looking surfaces for a split second, with or without the shader fixes.
- On a 12 GB card, video memory overflows into shared memory and system RAM fills up: 32 GB of RAM
  is tight.
