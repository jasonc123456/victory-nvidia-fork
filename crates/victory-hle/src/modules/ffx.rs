//! `amd_fidelityfx_loader_xbox.dll`: the upscale dispatch.
//!
//! The title's Series S mode renders below its output size and calls the
//! console's FidelityFX upscaler to make the output. The loader and the
//! upscaler are the title's own DLLs and run as they are, but the
//! upscaler's passes are compute pipelines in a container we do not
//! translate: its output stayed black. `ffxDispatch` alone is taken here:
//! an upscale dispatch becomes a plain stretch of the colour input
//! (`victory_gpu::list::upscale_stretch`); anything else, and an upscale we
//! could not stretch, goes to the title's DLL.
//!
//! The structures are the public FidelityFX API's (`ffx_api.h`,
//! `ffx_upscale.h`): a header of a type and a next pointer, and resources of
//! a pointer, a 32-byte description and a state.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// The DLL whose `ffxDispatch` is taken.
pub const LOADER: &str = "amd_fidelityfx_loader_xbox.dll";

static REAL: AtomicUsize = AtomicUsize::new(0);

/// The title's own `ffxDispatch`, for what is not stretched here.
pub fn set_real_dispatch(address: usize) {
    REAL.store(address, Ordering::Relaxed);
}

#[repr(C)]
struct Header {
    kind: u64,
    next: *const Header,
}

#[repr(C)]
struct Resource {
    resource: *const c_void,
    description: [u32; 8],
    state: u32,
}

#[repr(C)]
struct Upscale {
    header: Header,
    list: *const c_void,
    colour: Resource,
    depth: Resource,
    motion: Resource,
    exposure: Resource,
    reactive: Resource,
    composition: Resource,
    output: Resource,
    jitter: [f32; 2],
    motion_scale: [f32; 2],
    render: [u32; 2],
    upscale: [u32; 2],
    // The rest of `ffxDispatchDescUpscale` (`ffx_upscale.h`), for DLSS.
    enable_sharpening: u8,
    sharpness: f32,
    frame_time_delta: f32,
    pre_exposure: f32,
    reset: u8,
    camera_near: f32,
    camera_far: f32,
    camera_fov: f32,
    view_to_meters: f32,
    flags: u32,
}

/// `VICTORY_UPSCALE=dlss`: NVIDIA DLSS instead of the stretch, when the
/// build has it (the `dlss` feature) and the card does; else the stretch.
fn want_dlss() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_UPSCALE").is_some_and(|v| v == "dlss"))
}

const _: () = assert!(size_of::<Resource>() == 48);
const _: () = assert!(std::mem::offset_of!(Upscale, output) == 0x138);

/// `FFX_API_RETURN_ERROR`: what a dispatch nothing could answer returns.
const ERROR: u32 = 1;

/// `FFX_API_DISPATCH_DESC_TYPE_UPSCALE`.
const UPSCALE: u64 = 0x0001_0001;

type Dispatch = extern "system" fn(*mut *mut c_void, *const Header) -> u32;

extern "system" fn dispatch(context: *mut *mut c_void, desc: *const Header) -> u32 {
    // SAFETY: the title passes a dispatch description, whose header says
    // what follows it.
    if let Some(header) = unsafe { desc.as_ref() } {
        static SEEN: AtomicU32 = AtomicU32::new(0);
        let first = SEEN.fetch_add(1, Ordering::Relaxed) < 4;
        if header.kind == UPSCALE {
            // SAFETY: an upscale description, by its type.
            let d = unsafe { &*desc.cast::<Upscale>() };
            if first {
                tracing::info!(
                    list = ?d.list,
                    colour = ?d.colour.resource,
                    colour_size = ?(d.colour.description[2], d.colour.description[3]),
                    output = ?d.output.resource,
                    output_size = ?(d.output.description[2], d.output.description[3]),
                    render = ?d.render,
                    jitter = ?d.jitter,
                    upscale = ?d.upscale,
                    "upscale dispatch"
                );
            }
            note_textures(d);
            #[cfg(windows)]
            if victory_gpu::list::upscale(
                d.list,
                &victory_gpu::list::Upscale {
                    colour: d.colour.resource as u64,
                    depth: d.depth.resource as u64,
                    motion: d.motion.resource as u64,
                    exposure: d.exposure.resource as u64,
                    output: d.output.resource as u64,
                    render: d.render,
                    jitter: d.jitter,
                    motion_scale: d.motion_scale,
                    reset: d.reset != 0,
                    pre_exposure: d.pre_exposure,
                    near: d.camera_near,
                    far: d.camera_far,
                    fov: d.camera_fov,
                    frame_ms: d.frame_time_delta,
                    // ffxCreateContext is the title's own: its flags are not
                    // known here (DLSS's VICTORY_DLSS_MV_JITTERED and the like
                    // say what they would).
                    create_flags: 0,
                    dlss: want_dlss(),
                },
            ) {
                count(STRETCHED);
                return 0;
            }
            count(NOT_STRETCHED);
            if first {
                tracing::warn!("upscale dispatch not stretched: left to the title's upscaler");
            }
        } else {
            count(OTHER);
            note_kind(header.kind);
            if first {
                tracing::info!(
                    kind = format_args!("{:#x}", header.kind),
                    "another FidelityFX dispatch"
                );
            }
        }
    }
    match REAL.load(Ordering::Relaxed) {
        // Neither stretched nor passed on: the output was not written, and
        // the title is told so instead of presenting it as it is.
        0 => {
            static TOLD: std::sync::Once = std::sync::Once::new();
            TOLD.call_once(|| {
                tracing::error!(
                    "a FidelityFX dispatch was not answered: no stretch was recorded and the title's own ffxDispatch was not found"
                );
            });
            ERROR
        }
        // SAFETY: the title's own `ffxDispatch`, with its own arguments.
        real => unsafe { std::mem::transmute::<usize, Dispatch>(real)(context, desc) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;

    /// The upscale description as the public API lays it out: what the
    /// stretch reads sits where the title wrote it.
    #[test]
    fn the_upscale_description_is_laid_out_as_the_public_api() {
        assert_eq!(size_of::<Header>(), 16);
        assert_eq!(offset_of!(Upscale, list), 0x10);
        assert_eq!(offset_of!(Upscale, colour), 0x18);
        assert_eq!(offset_of!(Upscale, depth), 0x48);
        assert_eq!(offset_of!(Upscale, motion), 0x78);
        assert_eq!(offset_of!(Upscale, output), 0x138);
        assert_eq!(offset_of!(Upscale, jitter), 0x168);
        assert_eq!(offset_of!(Upscale, motion_scale), 0x170);
        assert_eq!(offset_of!(Upscale, render), 0x178);
        assert_eq!(offset_of!(Upscale, upscale), 0x180);
        assert_eq!(offset_of!(Upscale, sharpness), 0x18c);
        assert_eq!(offset_of!(Upscale, pre_exposure), 0x194);
        assert_eq!(offset_of!(Upscale, reset), 0x198);
        assert_eq!(offset_of!(Upscale, camera_near), 0x19c);
        assert_eq!(offset_of!(Upscale, flags), 0x1ac);
        assert_eq!(offset_of!(Resource, description), 8);
        assert_eq!(offset_of!(Resource, state), 40);
    }

    /// A dispatch that is not an upscale, with no DLL of the title's to
    /// pass it to, is an error and not a silent success.
    #[test]
    fn a_dispatch_nothing_answers_is_an_error() {
        let before = REAL.swap(0, Ordering::Relaxed);
        let other = Header {
            kind: 0x0002_0001,
            next: std::ptr::null(),
        };
        assert_eq!(dispatch(std::ptr::null_mut(), &other), ERROR);
        assert_eq!(dispatch(std::ptr::null_mut(), std::ptr::null()), ERROR);
        REAL.store(before, Ordering::Relaxed);
    }
}

/// What the title's import of `ffxDispatch` is bound to. The DLL is not an
/// HLE module: the title's own must load, for its other entry points and
/// for the dispatches passed on.
pub fn dispatch_address() -> usize {
    dispatch as *const () as usize
}

// Debugging a picture that stops being renewed: what the dispatches were,
// a line every two seconds of them, and each change of what an upscale reads
// and writes.
const STRETCHED: usize = 0;
const NOT_STRETCHED: usize = 1;
const OTHER: usize = 2;
static COUNTS: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];

fn count(which: usize) {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    COUNTS[which].fetch_add(1, Ordering::Relaxed);
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    match *last {
        None => *last = Some(now),
        Some(t) if now.duration_since(t) >= Duration::from_secs(2) => {
            let [s, n, o] = [0, 1, 2].map(|i| COUNTS[i].swap(0, Ordering::Relaxed));
            tracing::info!(
                stretched = s,
                not_stretched = n,
                other = o,
                seconds = format_args!("{:.1}", now.duration_since(t).as_secs_f32()),
                "FidelityFX dispatches since the last line"
            );
            *last = Some(now);
        }
        Some(_) => {}
    }
}

fn note_textures(d: &Upscale) {
    use std::sync::Mutex;
    static LAST: Mutex<(usize, usize, [u32; 2])> = Mutex::new((0, 0, [0, 0]));
    static TOLD: AtomicU32 = AtomicU32::new(0);
    let now = (d.colour.resource as usize, d.output.resource as usize, d.render);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if *last != now {
        if TOLD.fetch_add(1, Ordering::Relaxed) < 200 {
            tracing::info!(
                colour = format_args!("{:#x}", now.0),
                output = format_args!("{:#x}", now.1),
                render = ?now.2,
                list = ?d.list,
                "an upscale reads or writes other textures than the one before"
            );
        }
        *last = now;
    }
}

fn note_kind(kind: u64) {
    use std::sync::Mutex;
    static KINDS: Mutex<Vec<u64>> = Mutex::new(Vec::new());
    let mut kinds = KINDS.lock().unwrap_or_else(|e| e.into_inner());
    if !kinds.contains(&kind) {
        kinds.push(kind);
        tracing::info!(kind = format_args!("{kind:#x}"), "a new kind of FidelityFX dispatch");
    }
}
