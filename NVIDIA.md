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
  stripes) are caught about four times as often with `120`, at no frame time measured.
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

    set "MORE=VICTORY_UP_QUEUE=1 VICTORY_JOIN_BARRIERS=1 VICTORY_RECHECK_EVERY=120 VICTORY_TEXTURE_ROOM_PERCENT=95 VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_LEVELS_ON_DEMAND=1"

- `VICTORY_UP_QUEUE=1 VICTORY_JOIN_BARRIERS=1`: about 7% faster together.
- `VICTORY_RECHECK_EVERY=120`: textures the title changes late are caught (above).
- `VICTORY_TEXTURE_ROOM_PERCENT=95`: textures may fill 95% of the card's budget instead of 90%.
  On a 12 GB card, the passes that found no room for a texture fell from 19,341 to 181, and
  objects that were drawn black got their textures, at the same frame rate. Lower values (or
  `LOW_VRAM=1`, which sets 70) only make more textures wait.
- `VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_LEVELS_ON_DEMAND=1` (from the launcher of
  codec-d/cool_fox): title buffers from 64 KiB up get a copy in the card's memory, so shaders
  stop reading them over PCIe, and textures take the card's memory level by level. On a 12 GB
  card: 6.4 to 7.3 fps outside, the card no longer waited for, more draws kept, textures as
  before. The card's memory is then full, and about 0.8 GB of 32 GB RAM stays free.

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
| `SAFE=0` on its own (release, with the memory switches) | 6.9 fps against 7.3: preparing 108 ms instead of 117, but the card is waited for again (13 ms, up to 48) and 1 GB more memory is used (0.5 GB of RAM left). The welcome screen steps and textures look worse |

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
  title changes after the layer stopped looking. `VICTORY_RECHECK_EVERY=120` catches them sooner.
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
