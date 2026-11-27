# VIctory on an RX 7900 XTX

> **victory-nvidia-fork:** this copy adds fixes for NVIDIA cards, a launcher script and a log
> collector. See [NVIDIA.md](NVIDIA.md) for the changes, the results on NVIDIA cards, and how to
> make your own start file.
>
> **Latest (2026-11-27), on an RTX 5070 12 GB with an i7-14700K and 32 GB of DDR4-3200, Series
> S mode, ray tracing off:**
>
> - **NVIDIA DLSS** does the title's upscale (`VICTORY_UPSCALE=dlss`, 1280x720 to 1920x1080, from
>   aleksesipenko/VIctory-RTX): a sharper picture, 9.6 fps median against 8.6 with the title's own
>   upscaler.
> - **NVIDIA DLSS Frame Generation** (`VICTORY_FRAME_GEN=2`, `3` or `4`, through NVIDIA
>   Streamline with Reflex, from T0nyyyyy/Nvidia-framegenmaybefixed-): at 3x the title makes
>   7.4-8 frames a second in busy places and about 22 reach the screen. Motion looks smooth; the
>   controls still answer at the title's own rate. RTX 40 or newer (2x), RTX 50 for 3x and 4x.
> - **No more sky and water flashes near dense places:** NVIDIA caps the view heap at 1,000,000
>   views (the title asks 1,245,184), and the slots for the tables made each frame ran out, so
>   draws were dropped. The launcher now gives them half the heap (`VICTORY_DYNAMIC_SLOTS=524288`):
>   big flashes 28 to 1 in a run near the first house, at the same frame rate.
>
> NVIDIA's DLSS and Streamline files are not in this repository: [NVIDIA.md](NVIDIA.md) says
> where to get them and where to put them. The frame is still bound by the processor side
> (preparing about 100 ms, copying back 35-45 ms a frame), not by the card.
>
> **On NVIDIA (2026-11-26):** about 7 fps in the world on an RTX 5070 12 GB (ray tracing, Series
> S mode), up from 4.5-5.7 before this fork took in upstream's latest processor-side work. The
> frame is bound by the processor side's preparing (130-160 ms a frame before that), not by the
> card. A 12 GB card still spills about 8 GB into shared memory. Since 2026-11-26 the fork also
> carries the NVIDIA shader fixes of fxceless's pull requests (32-lane waves): on an RTX 5070,
> every compute program of the title now gets a pipeline (29 were refused before), at the same
> 6-7 fps once the first start has rebuilt its pipelines.
>
> **Later on 2026-11-26:** with ray tracing off, an RTX 5070 runs 30-45% faster: 5.5-7.4 fps
> outside (4.5-5 with it), about 10 indoors. The card then works only 3-47 ms a frame; the
> processor side is the whole frame. `VICTORY_RECHECK_EVERY=120` (new here) and
> `VICTORY_TEXTURE_ROOM_PERCENT=95` fix stale and missing textures at no measured cost.
> Upstream's commits after 9c23e8d, its renderer patches and P-core pinning were tried and not
> kept: all are measured in [NVIDIA.md](NVIDIA.md).
>
> **Later still:** the release build (`RELEASE=1`) prepares 10-15% less than the diagnostics
> build. With `VICTORY_CARD_BUFFERS_MIN_KB=64 VICTORY_LEVELS_ON_DEMAND=1` (from codec-d/cool_fox's
> launcher) an RTX 5070 runs about 7.3 fps outside (median), up from 6.4, and the card is no longer
> waited for (`VICTORY_LEVELS_ON_DEMAND` was later left out: it left roads without their
> texture). `SAFE=0` stays slower on 32 GB of RAM. Everything below this box is upstream's own
> description of the RX 7900 XTX fork.

A fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files the user supplies.
This one is about **speed on AMD RDNA 3**: the world at 30 frames a second on
one PC, without giving up the picture. It is not there yet.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Where it stands (2026-11-25)

Tested on one machine only: Windows 11, Intel i9-12900K, 64 GB RAM, Radeon
RX 7900 XTX, 1440p, the Series X profile.

| | |
|---|---|
| Menus, videos, loading screens | 30 fps |
| Prologue, indoors | 50-75 ms a frame (13-20 fps) |
| First mission with the boat | 46-53 ms (19-22 fps) |
| Street chase with many cars and people | 100-165 ms (6-10 fps), single frames far longer |
| Outdoors on foot, the camp | 60-90 ms |
| Flying | 42-57 ms |
| Longest session so far | 41 minutes, no crash |
| Memory | 40-46 GB of private memory: **64 GB of RAM recommended** |

It started at 280 ms a frame in the same scenes two days earlier.

What is known to be wrong:

- Trees, plants and far buildings flicker outdoors in some places.
- Rooms with little direct light are too dark without `-RayTracing`.
- Textures of what just came into view appear late while the title streams.
- The first run with a new driver or a new build stutters while the driver makes
  pipelines (`victory warm` makes them ahead).
- Recording a frame in the world makes the title's own watchdog end it.

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone https://github.com/avatarmanz/VIctory-7900XTX
cd VIctory-7900XTX
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path of your copy's executable>
```

`scripts\play.ps1 -Safe` leaves out the newest, faster ways; try it first when
something is wrong. `-RayTracing` turns the title's ray-traced indirect light on.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
```

Commands, debugging switches and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## What is different here

In short; the reasons and the measurements are in [docs/findings.md](docs/findings.md).

- **List calls are written down, not carried out, on the title's threads**
  (`VICTORY_THIN`): eight threads of ours translate them.
- **Submissions are prepared by a thread of ours and joined**
  (`VICTORY_ASYNC_SUBMIT`, `VICTORY_JOIN_PARTS`), with what can be spread over
  threads spread: refreshes, compares (AVX2), descriptor records, landings.
- **What shaders wrote comes back beside the write guards**, by the dwords that
  changed, on a thread of its own (`VICTORY_RELAY`, `VICTORY_LAND`).
- **Buffer fills that need bytes still on the card are done on the card**
  (`VICTORY_CARD_STEPS`) instead of waiting for it.
- **The title's writes are caught more cheaply**: one view opened at a fault,
  blocks that rest, places in the title's code that look before they write.
- **Textures go up on a copy queue of their own** and a slot shows one only after
  its upload ran, which also ended a device loss that came every 15 minutes or so.
- **Boats float**: textures that shaders write and the title's CPU reads go back
  to title memory (from fxceless and Geckomacabre, on by default here).
- **Frame record and replay** with per-pass times and shader variants compared
  turn by turn ([docs/replay.md](docs/replay.md)).

## Credits

This is a tree of other people's work with more on top. The history is kept, and
commits taken from elsewhere name the commit and its author.

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end and
  the shader recompiler.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the fork
  this one started from, and the people whose pull requests it carries.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: ray tracing fixes,
  cube gradients, sparse textures.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  the ambient light idea, pipeline workers, and work by romainhedouin.
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**.
- **[myokex/victory-amd](https://github.com/myokex/victory-amd)**: the guard over
  every view of a page (with Spexivus, joroyt and tightfall), and Joshua Odom's
  reads into title memory.
- **Geckomacabre** and **cxy0072019-sketch**: the texture read-back.
- **[aleksesipenko/VIctory-RTX](https://github.com/aleksesipenko/VIctory-RTX)**:
  NVIDIA DLSS in place of the title's upscale.
- **[T0nyyyyy/Nvidia-framegenmaybefixed-](https://github.com/T0nyyyyy/Nvidia-framegenmaybefixed-)**:
  DLSS Frame Generation through NVIDIA Streamline, and the dynamic descriptor slots on
  NVIDIA's capped view heap.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only) was read for how the same
  title behaves under another layer; no code of it is here.
- This fork's changes were made with [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).
