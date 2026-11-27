# victory-nvidia-fork: VIctory on NVIDIA

A fork of the VIctory-7900XTX fork (avatarmanz/VIctory-7900XTX, itself from fxceless/victory-fork),
with changes for NVIDIA cards. It holds no title files: you supply your own.

Upstream's work is merged in up to avatarmanz 9c23e8d (2026-11-26). It includes the processor-side
speedups (`VICTORY_SYNC_IN_PLACE`, `VICTORY_SPANS_QUICK`, `VICTORY_METER_THREAD`, `VICTORY_SEND_OWN`,
`VICTORY_SHOW_NEW_LATER`, all on) and switches still off by default, to try through `MORE=`:
`VICTORY_UP_QUEUE=1`, `VICTORY_JOIN_BARRIERS=1`, `VICTORY_LANE_CORE=1`, `VICTORY_PORT_CONSTANTS=1`.

Upstream's later commits (avatarmanz 9c23e8d..9576c45, 73 commits) are **not** merged: on an
RTX 5070 that tree ran at 2-3 fps where this one runs at 4.5-5 (same save, same place), and it
drew a character without a face. That is so on upstream's own tree too, without this fork's
changes, so it is not a merge fault. See *Tried on an RTX 5070* below.

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
- **Textures looked at again** (`VICTORY_RECHECK_EVERY=<frames>`, off by default): a texture the
  layer had stopped watching after its first few looks is compared again every so many frames,
  and sent up again when the title has changed it. Stale or half-new textures (blocky patches,
  stripes) are caught about four times as often with `120`, and twice as often again with `30`,
  at no frame time measured.
- **Program switches for looking into the picture** (all off by default):
  `VICTORY_SHADER_NAMES=1` logs the name of each program of the title once, as it is created;
  `VICTORY_PROGRAM_RUNS=1` logs each compute program's first run and, every 30 s, which ran and
  how often; `VICTORY_LEAVE_OUT=<part>,...` drops the dispatches of compute programs whose name
  holds a part (or whose hash is one). Both take a text instead of `1` to report only names that
  hold it.
- **Voxel light with ray tracing off** (`VICTORY_SWAP=<from>:<to>,...`; the launcher sets it when
  `RAYTRACING=0`): with ray tracing off the title still traces its screen probes against the
  ray-tracing structure, and those rays all miss, leaving blocks and speckle in every unlit place.
  The switch binds the title's own voxel trace of that pass in its place (the first of its
  programs whose name holds `to`, with the title's resources and dispatch sizes). The blocks are
  gone; it costs about a quarter of the frame rate (see *Results*). `-Settings
  'VICTORY_SWAP=none'`, or `MORE=VICTORY_SWAP=none`, keeps the title's own.
- **Card writes left on the card** (`VICTORY_SKIP_WC_BACK=1`, off by default; from
  fxceless/victory-fork PR 868 by Chipsraine): what shaders write to title memory the title
  reserved write-combining (memory its processor writes and does not read back) is not copied back
  from the card. That is most of what each frame copies back. Two rules keep the picture right:
  declared ranges under `VICTORY_SKIP_WC_MIN_KB` (256) are always copied back, and so are pages the
  layer has sent up to the card more than once (`VICTORY_SKIP_WC_TITLE_PAGES`, on), because
  the title writes those itself. Without the two rules the whole picture flashed every frame.
  `VICTORY_CARD_ONLY=<name part>,...` keeps one named program's buffer writes on the card instead.
- **Light stand-in without ghosts** (`VICTORY_AMBIENT_CLEAN`, on by default; `0` turns it off):
  the stand-in raises the light in the image the title's screen-probe resolve leaves, and the
  title reads that image back as the next frame's history. Its estimate follows the screen, so
  after a turn the old view's light stayed in the history for seconds: a see-through ghost of the
  old view, and the world seemed to jump back a frame. The image is now copied before the raise
  and put back before the next frame's resolve, so the raise shows in its own frame only.
- **Textures the title keeps rewriting, looked at every frame** (`VICTORY_LIVE_TEXTURES`, on by
  default; `0` turns it off; avatarmanz pull request #390): a texture found changed after its
  upload is fingerprinted whole (up to 4 MiB) every frame until it rests for 120 frames.
- **Output size, render scale and upscaler** (`VICTORY_OUTPUT`, `VICTORY_RENDER_SCALE`,
  `VICTORY_FFX_SCALE`, `VICTORY_UPSCALER`; each off without it; from avatarmanz, with pull
  request #53's picker and scale): a few places in the title's code changed at load, each
  compared byte for byte first and left alone if it differs.
- **NVIDIA DLSS in place of the upscale stand-in** (`VICTORY_UPSCALE=dlss`, Series S; from
  aleksesipenko/VIctory-RTX): the title's FidelityFX upscale is done by DLSS from its colour,
  depth, motion vectors and jitter, instead of a plain stretch; if DLSS cannot run, the stretch.
  NVIDIA's files are not in this repository: put `nvsdk_ngx_d.lib`
  (`lib\Windows_x86_64\x64`) and `nvngx_dlss.dll` (`lib\Windows_x86_64\rel`) from
  <https://github.com/NVIDIA/DLSS> (NVIDIA's licence) in `%LOCALAPPDATA%\VIctory-Fork\nvidia-dlss`
  (or name the folder in `VICTORY_DLSS_SDK`); the launcher then builds with DLSS. Check the DLL's
  NVIDIA signature (file properties, Digital Signatures). Switches: `VICTORY_DLSS_PRESET`
  (`J`, `K`, `L`, `M`), `VICTORY_DLSS_QUALITY`, `VICTORY_DLSS_MV_JITTERED`, `VICTORY_DLSS_MV_SIGN`,
  `VICTORY_DLSS_JITTER_SIGN`, `VICTORY_DLSS_SHARPNESS`.
- **NVIDIA DLSS Frame Generation** (`VICTORY_FRAME_GEN=2`, `3` or `4`: frames shown per frame the
  title makes; from T0nyyyyy/Nvidia-framegenmaybefixed-): NVIDIA Streamline's proxy swap chain
  makes the frames in between at each present, from the depth, motion vectors, jitter and depth
  range of the title's upscale, with Reflex. RTX 40 or newer (2x), RTX 50 for 3x and more. The
  window's title shows both rates. NVIDIA's files are not in this repository: put the `bin\x64`
  folder of the Streamline SDK release (<https://github.com/NVIDIA-RTX/Streamline/releases>,
  NVIDIA's licence) in `%LOCALAPPDATA%\VIctory-Fork\streamline` (or name it in
  `VICTORY_STREAMLINE_DIR`); the launcher finds it. Check the DLLs' NVIDIA signatures. Turn NVIDIA
  Smooth Motion off for `victory.exe` with it. Switches: `VICTORY_FG_BUFFERS` (swap-chain buffers,
  2 by default), `VICTORY_FG_TITLE_FOV=1` (the title's field of view, not 60 degrees),
  `VICTORY_FG_MV_JITTERED=0`, `VICTORY_FG_REFLEX=0`, `VICTORY_FG_ANY_UPSCALE=0`,
  `VICTORY_DLSS_BEFORE_SL=1`. It makes the motion look smoother; the controls still answer at the
  title's own rate, and a frame that flashes is shown for longer.
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
   set RELEASE=1
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
| `RAYTRACING`    | 0 or 1    | Ray-traced light (lit shadows). 0 is 30-45% faster on a 12 GB card; rooms are darker |
| `SEPARATE_PADS` | 1         | Each controller is its own device                                  |
| `SAFE`          | 1         | Without the fork's newest CPU shortcuts. Steadier, and no welcome-screen stepping. 0 was slower on an i7-14700K with 32 GB |
| `SERIES_X`      | 0         | Series X mode instead of Series S. Needs more video memory         |
| `LOW_VRAM`      | auto      | Less video memory for textures and copies. auto: on for 8 GB cards or smaller |
| `RELEASE`       | 1         | Release build instead of the diagnostics build: 10-15% less preparing on the processor side. 0 keeps the device-removed breadcrumbs, for a crash to look into |
| `INPUT_TRACE`   | 0         | Log every controller reading                                       |
| `MORE`          | see below | Further settings, space-separated                                  |

Suggested `MORE` on an RTX 5070 12 GB (each measured, see *Results*):

    set "MORE=VICTORY_UP_QUEUE=1 VICTORY_JOIN_BARRIERS=1 VICTORY_RECHECK_EVERY=30 VICTORY_TEXTURE_ROOM_PERCENT=95 VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_SKIP_WC_BACK=1 VICTORY_ASYNC_SUBMIT=1 VICTORY_UPSCALER=6"

- `VICTORY_UP_QUEUE=1 VICTORY_JOIN_BARRIERS=1`: about 7% faster together.
- `VICTORY_RECHECK_EVERY=30`: textures the title changes late are caught (above); 30 caught
  twice as many as 120 at the same frame rate.
- `VICTORY_TEXTURE_ROOM_PERCENT=95`: textures may fill 95% of the card's budget instead of 90%.
  On a 12 GB card, the passes that found no room for a texture fell from 19,341 to 181, and
  objects that were drawn black got their textures, at the same frame rate. Lower values (or
  `LOW_VRAM=1`, which sets 70) only make more textures wait.
- `VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_LEVELS_ON_DEMAND=1` (from the launcher of
  codec-d/cool_fox): title buffers from 64 KiB up get a copy in the card's memory, so shaders
  stop reading them over PCIe, and textures take the card's memory level by level. On a 12 GB
  card: 6.4 to 7.3 fps outside, the card no longer waited for, more draws kept, textures as
  before. The card's memory is then full, and about 0.8 GB of 32 GB RAM stays free. Rooms look
  the same with or without it: how dark they are follows the hour of the title's day (below).
  **Leave `VICTORY_LEVELS_ON_DEMAND` out:** with it the roads lost their texture and lane lines
  (black asphalt, in sun and shade), and the card was waited for again with the title's own
  upscaler. `VICTORY_CARD_BUFFERS_MIN_KB=64` alone keeps the speed: without either, the card
  waited about 200 ms a frame (3.4 fps).
- `VICTORY_SKIP_WC_BACK=1`: with the voxel light on, 6.3 to 7.4 fps in lighter views and 5.7 to
  6.9 in busy ones (short runs, so rough figures), copy-back 50-65 ms to 31-39. No flashing with
  its two rules. An experiment: it assumes the title never reads that memory back.
- `VICTORY_ASYNC_SUBMIT=1`: submissions are prepared by a thread of the layer's own while the
  title's thread goes on (it was on only with `SAFE=0`). With `SAFE=1`: 7.2 to 8.1 fps in lighter
  views and 5.9 to 7.2 in busy ones; no new faults.
- `VICTORY_UPSCALER=6`: Series S uses the title's own temporal upscaler instead of FidelityFX,
  which this layer can only stretch: a much sharper picture from the same 1280x720. The first
  start after it builds its pipelines (slow); after that about the speed of the stretch.
  `VICTORY_RENDER_SCALE=1.0` on Series S stops the title at its own check while it makes its
  targets; use it on Series X only.

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
| RTX 5070 12 GB, i7-14700K, 32 GB             | outside, one save and place (about 6,000-11,000 draws): ray tracing on / off | On: 4.5-5 fps, about 210 ms a frame. **Off: 5.5-7.4 fps, 144-158 ms** (about 10 fps indoors). With ray tracing off the card works only 3-47 ms a frame: the frame is the processor side's preparing (about 105 ms) and copying back (about 42 ms) |
| RTX 5070 12 GB, i7-14700K, 32 GB             | ray tracing off, with `VICTORY_RECHECK_EVERY=120` and `VICTORY_TEXTURE_ROOM_PERCENT=95` | Same frame rate; stale textures caught about 4 times as often; textures left without room 181 times instead of 19,341, and black objects textured again |
| RTX 5070 12 GB, i7-14700K, 32 GB             | the same, `RELEASE=1` instead of the diagnostics build (11 minutes outside) | Preparing 99 ms instead of 103-121, copying back 40 instead of 44-50: 10-15% less on the processor side. 6.4 fps median over the run, 8-9 fps more often; no new warnings |
| RTX 5070 12 GB, i7-14700K, 32 GB             | release, with `VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_LEVELS_ON_DEMAND=1` | **7.3 fps median outside, 138 ms** (from 6.4 and 155). The card is never waited for (0 ms, from 17); about 9,800 draws a frame, about 200 dropped (from 8,000 and 1,700). Textures look the same; the card's memory is full (11.5 of 12 GB, 7.8 GB shared) |
| RTX 5070 12 GB, i7-14700K, 32 GB             | the same, `VICTORY_RECHECK_EVERY=30` instead of 120 | Stale textures caught about twice as fast (204 in 75 s against 208 in 170 s); 133 ms a frame at 6,000-8,000 draws against 138-147. Blocky patches on clothes in unlit places remain (they are not textures, see *Known faults*) |
| RTX 5070 12 GB, i7-14700K, 32 GB DDR4 at 3200 MHz | the same, the memory set to 3200 MHz | 8.3 fps median over a run (121 ms), 9-9.5 in lighter views; preparing about 20% shorter (90-103 ms). The frame is bound by the processor and memory |
| RTX 5070 12 GB, i7-14700K, 32 GB DDR4 at 3200 MHz | the same, with the voxel light (`VICTORY_SWAP`, now the default without ray tracing) | **No blocks or speckle in unlit places.** 6.3 fps (158 ms) in lighter views, 5.7 (176 ms) in busy ones, against 8.3 and 8.1: copying back 50-65 ms (20 MB a frame, from 10), and in busy views the card is waited for (65 ms) |
| RTX 5070 12 GB, i7-14700K, 32 GB DDR4 at 3200 MHz | voxel light, `VICTORY_SKIP_WC_BACK=1`, the stand-in without ghosts | **7.4 fps (135 ms) in lighter views**, 5.9 (170 ms) in busy ones; no blocks in unlit places, no ghosts when turning. The world still jumps back now and then in dense places (near the first house), never in open country |
| RTX 5070 12 GB, i7-14700K, 32 GB DDR4 at 3200 MHz | the suggested `MORE` above (no `LEVELS_ON_DEMAND`; async submit; the title's own upscaler), live textures, voxel light | **8.5 fps median in busy views (118 ms), 9-11 on the road**; the card not waited for; roads textured, a sharp picture, no ghosts or jumps, the sky flashing once or twice a session |
| RTX 5070 12 GB, i7-14700K, 32 GB DDR4 at 3200 MHz | the same with `VICTORY_UPSCALE=dlss` in place of `VICTORY_UPSCALER=6` | **9.6 fps median (8.7-10.0 most of the time) against 8.6 (7.6-9.6)**; feels smoother; DLSS 310.9.16, 1280x720 to 1920x1080 |
| RTX 5070 12 GB, i7-14700K, 32 GB DDR4 at 3200 MHz | the same with `VICTORY_FRAME_GEN=2`, then `3` (Streamline 2.14.16) | 2x: 8.1 fps made, about 15 shown; 3x: 7.4 made, about 22 shown, smooth to the eye; at 3x the card was waited for 28 ms a frame (10 ms with `VICTORY_FG_BUFFERS=3`). The sky and water flashes stand out more |

With the shader fixes, the first start after an update rebuilds every pipeline while it plays
(hundreds at first, 1.5-4 fps for a minute or two); the second start loads them from the cache.
`LOW_VRAM=1` on a 12 GB card was slower than `auto` (4.2 fps), as was `SAFE=0` (with it and on its own).

### Tried on an RTX 5070, and not kept

Each in the same place as the results above, against this tree:

| Tried | Result |
|---|---|
| Upstream to avatarmanz 9576c45, merged or on its own | 2-3 fps (340-580 ms a frame) against 4.5-5; a character drawn without a face; more speckle. The cause is not its 5 new ray-traced shadow programs or its two-plane compose (both switched in one run, no change) |
| Renderer patch 1 `VICTORY_PORT_CONSTANTS=1` and patch 2 `VICTORY_PORT_DESCRIPTORS=1` (upstream 9997d25, 4cb90be) | Preparing halves, but the card reads the constants over PCIe: card time 88-163 ms with ray tracing, 105-233 ms without (from 3-47). Same fps with ray tracing, slower and a broken picture without (black sky, dark blotches). Patch 2 also misses 1,000-5,000 view records a run, which its fallback has to make |
| The whole process on the P-cores only (i7-14700K) | No faster: preparing 3-5 ms shorter, but the title's own threads lose the E-cores |
| Less texture room (`LOW_VRAM=1`, 70%) | Slower, and more textures wait |
| Ray-tracing hit resolves and ray activation left out with ray tracing off (after codec-d/cool_fox) | 8.5-10 fps, but the colours break: purple grass and trees, near-black shade, sparkles. With ray tracing off these programs still write the light the rays fall back to; the speed was that work skipped |
| A DirectStorage staging buffer of 256 MB instead of the title's 32 (after codec-d/cool_fox) | Slower: 6.5 fps against 7.8 at the same spot, preparing 132 ms instead of 103 and copying back 60 instead of 42 |
| `SAFE=0` on its own (release, with the memory switches) | 6.9 fps against 7.3: preparing 108 ms instead of 117, but the card is waited for again (13 ms, up to 48) and 1 GB more memory is used (0.5 GB of RAM left). The welcome screen steps and textures look worse |
| `VICTORY_INDIRECT_REST=0` with `VICTORY_AMBIENT_AIR=1.5` | Same frame rate. Rooms fully black at night with the air bound (the haze it takes away was the only fill light), and the faults on clothes unchanged |
| `VICTORY_CARD_BUFFERS_MIN_KB=1024`, or the switch left out (16 MiB) | 1024: 5.7 fps (176 ms), the card waited for 53 ms a frame. Left out: 6.8 fps (147 ms), card 9 ms. 64 stays: 7.2-7.5 fps, card 2-3 ms |
| `VICTORY_AMBIENT=constant:0.3` (one level of ambient light everywhere) | 5.8 fps (173 ms), the card waited for 43 ms; houses near black, water flashing white. The patches on clothes stay, so they do not come from the ambient stand-in |
| The three busiest of the 17 variants of the title's ray-hit resolve left out (`VICTORY_LEAVE_OUT`, about 70% of their ~900 dispatches a frame) | No faster (147 ms against 132-145); colours outside fine, but the screen turns white for 10-20 s after walking indoors. These dispatches are cheap; the speed of the earlier row came from leaving out the ray activation |

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

- Green and red stripes, or a pixelated look, on some characters' arms and clothes: textures the
  title changes after the layer stopped looking. `VICTORY_RECHECK_EVERY=30` catches them sooner.
- Without the voxel light (`VICTORY_SWAP=none`, or with ray tracing on before its structure is
  built): black and white blocks and speckle on clothes, walls and furniture in unlit places, gone
  close to a lamp. They are in the indirect light, not in textures. With ray tracing off the title
  still runs the ray-traced variants of its light passes (screen probes, irradiance cache, ray-hit
  resolves; `VICTORY_PROGRAM_RUNS=1`), whose rays all miss; its voxel and fallback variants, made
  at load, never run on their own. What makes it pick one or the other is not known yet; the
  voxel light swaps the screen-probe trace, which carries the blocks.
- The voxel light costs about a quarter of the frame rate on its own: what its trace writes is
  copied back to title memory (about 10 MB more a frame). `VICTORY_SKIP_WC_BACK=1` wins most of
  that back.
- The sky flashing black and the water vanishing for a frame near dense places: two causes, both
  fixed. A small texture the title writes every frame was shown frames late
  (`VICTORY_LIVE_TEXTURES`, on by default, looks at such textures every frame). And NVIDIA caps the
  view heap at 1,000,000 views (the title asks 1,245,184), so the fixed 2^18 slots for the tables
  made each frame ran out in busy places: draws were dropped, indirect ones (the sky, the water)
  among them, a big flash every 1-4 s near the first house. `VICTORY_DYNAMIC_SLOTS=524288` (the
  launcher's default; at most half the heap) gives them half the heap: big flashes 28 to 1 in a
  run there, dropped indirect draws about 60 a frame to none, the same frame rate (from
  T0nyyyyy/Nvidia-framegenmaybefixed-).
- Rooms are dark at night and at dusk with ray tracing off: only lamps light them then.
- The welcome screen steps back and forth without `SAFE`.
- Grainy speckle on hair, walls, floors and in the air, with ray tracing on or off (lighting
  noise), and bright stair-step blocks near windows.
- Some textures stay blank: the title makes a few hundred texture views at load before their
  memory exists, and the layer gives those a null view (logged as `placed unordered texture view
  not mirrored`; they are shader views, not writes). A minor fault: about 1,000-2,000 such views
  in minutes of play.
- Far-away trees can be drawn in a flat bright green.
- Before the shader fixes, the driver refused compute pipelines with E_INVALIDARG (four in a run,
  29 over the whole pack), and each dropped its work. With them, every compute program gets a
  pipeline, and a run refuses none.
- Brief flashes of wrong-looking surfaces for a split second, with or without the shader fixes.
- On a 12 GB card, video memory overflows into shared memory and system RAM fills up: 32 GB of RAM
  is tight.
