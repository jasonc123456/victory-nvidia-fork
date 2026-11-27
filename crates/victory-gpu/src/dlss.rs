//! NVIDIA DLSS in place of the title's FidelityFX upscale (Series S,
//! `VICTORY_UPSCALE=dlss`, builds with the `dlss` feature). From
//! aleksesipenko/VIctory-RTX (GPL-3.0), in place of our plain stretch.
//!
//! The title renders below its output size and asks its upscaler to make the
//! output from the colour, depth and motion vectors it rendered, with the
//! camera's jitter: what DLSS takes as well. The title's own FidelityFX passes
//! are slow translated for 32-lane cards (about 150 ms of card time a frame on
//! an RTX 3070 Ti); DLSS runs on the card's tensor cores in a millisecond or
//! two.
//!
//! The SDK (NVIDIA/DLSS on GitHub: `include`, `lib/Windows_x86_64`) is not part
//! of this repository: the build links `nvsdk_ngx_d.lib` from
//! `VICTORY_DLSS_SDK`, and `nvngx_dlss.dll` is looked for in `VICTORY_DLSS_DIR`
//! (else beside the executable). Any failure leaves the caller to stretch.
//!
//! Switches: `VICTORY_DLSS_QUALITY` (0 performance .. 2 quality, 5 DLAA; auto
//! from render/output ratio if unset), `VICTORY_DLSS_PRESET` (`J`=10, `K`=11
//! [DLSS 4 Transformer], `L`=12 [DLSS 4.5 UltraPerf], `M`=13 [DLSS 4.5
//! Transformer], or `default`/unset), `VICTORY_DLSS_MV_SIGN=-1` (motion vectors
//! the other way), `VICTORY_DLSS_JITTER_SIGN=-1`,
//! `VICTORY_DLSS_MV_JITTERED=0|1` (default: from FFX create flags),
//! `VICTORY_DLSS_HDR=0|1` (default: by the colour format),
//! `VICTORY_DLSS_DEPTH_INVERTED=0|1` (default: by near > far),
//! `VICTORY_DLSS_SHARPNESS=<0..1>`.

use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::sync::{Arc, Mutex};

use windows::Win32::Graphics::Direct3D12::*;
use windows::core::Interface;

type Result = c_int;
const SUCCESS: Result = 1;
const FEATURE_SUPER_SAMPLING: c_int = 1;
const ENGINE_CUSTOM: c_int = 0;
const VERSION_API: c_int = 0x15;

const FLAG_HDR: c_int = 1 << 0;
const FLAG_MV_LOW_RES: c_int = 1 << 1;
const FLAG_MV_JITTERED: c_int = 1 << 2;
const FLAG_DEPTH_INVERTED: c_int = 1 << 3;
const FLAG_AUTO_EXPOSURE: c_int = 1 << 6;

#[repr(C)]
struct PathList {
    path: *const *const u16,
    length: c_uint,
}

#[repr(C)]
struct Logging {
    callback: *const c_void,
    minimum: c_int,
    disable_other_sinks: bool,
}

#[repr(C)]
struct CommonInfo {
    paths: PathList,
    internal: *const c_void,
    logging: Logging,
}

#[link(name = "nvsdk_ngx_d", kind = "static")]
unsafe extern "C" {
    fn NVSDK_NGX_D3D12_Init_with_ProjectID(
        project: *const c_char,
        engine: c_int,
        engine_version: *const c_char,
        data_path: *const u16,
        device: *mut c_void,
        info: *const CommonInfo,
        version: c_int,
    ) -> Result;
    fn NVSDK_NGX_D3D12_GetCapabilityParameters(out: *mut *mut c_void) -> Result;
    fn NVSDK_NGX_D3D12_CreateFeature(list: *mut c_void, feature: c_int, params: *mut c_void, out: *mut *mut c_void) -> Result;
    fn NVSDK_NGX_D3D12_ReleaseFeature(handle: *mut c_void) -> Result;
    fn NVSDK_NGX_D3D12_EvaluateFeature_C(list: *mut c_void, handle: *const c_void, params: *const c_void, callback: *const c_void) -> Result;
    fn NVSDK_NGX_Parameter_SetUI(p: *mut c_void, name: *const c_char, value: c_uint);
    fn NVSDK_NGX_Parameter_SetI(p: *mut c_void, name: *const c_char, value: c_int);
    fn NVSDK_NGX_Parameter_SetF(p: *mut c_void, name: *const c_char, value: f32);
    fn NVSDK_NGX_Parameter_SetD3d12Resource(p: *mut c_void, name: *const c_char, value: *mut c_void);
    fn NVSDK_NGX_Parameter_GetI(p: *mut c_void, name: *const c_char, out: *mut c_int) -> Result;
}

/// A parameter name as the SDK spells it.
macro_rules! name {
    ($s:literal) => {
        concat!($s, "\0").as_ptr().cast::<c_char>()
    };
}

/// What the title gives an upscale, as the host's resources.
pub(crate) struct Inputs<'a> {
    pub colour: &'a ID3D12Resource,
    pub depth: Option<&'a ID3D12Resource>,
    pub motion: Option<&'a ID3D12Resource>,
    pub exposure: Option<&'a ID3D12Resource>,
    pub output: &'a ID3D12Resource,
    pub jitter: [f32; 2],
    pub motion_scale: [f32; 2],
    pub render: [u32; 2],
    pub reset: bool,
    pub pre_exposure: f32,
    pub near: f32,
    pub far: f32,
    pub frame_ms: f32,
    /// `FFX_API_CREATE_CONTEXT_DESC_UPSCALE.flags` from `ffxCreateContext`.
    pub create_flags: u32,
}

struct Feature {
    handle: *mut c_void,
    /// render width and height the feature was made for, output size, flags
    key: [u32; 5],
}

// SAFETY: `handle` is only passed to NGX calls under `DLSS`; when the last
// `Arc<Feature>` drops (on whichever thread resets the last `HostList` that
// recorded it), `Drop` only queues the raw handle into `RETIRED_HANDLES` for
// `NVSDK_NGX_D3D12_ReleaseFeature` under `DLSS`.
unsafe impl Send for Feature {}
unsafe impl Sync for Feature {}

/// Feature handles whose cache slot and every recording `HostList` have been
/// released; drained strictly under `DLSS` so NGX calls stay serialized.
static RETIRED_HANDLES: Mutex<Vec<usize>> = Mutex::new(Vec::new());

impl Drop for Feature {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            RETIRED_HANDLES.lock().unwrap_or_else(|e| e.into_inner()).push(self.handle as usize);
        }
    }
}

/// Releases queued NGX feature handles. Caller must hold `DLSS`.
fn drain_retired() {
    let handles = {
        let mut q = RETIRED_HANDLES.lock().unwrap_or_else(|e| e.into_inner());
        if q.is_empty() {
            return;
        }
        std::mem::take(&mut *q)
    };
    for raw in handles {
        // SAFETY: `DLSS` mutex is held; handle came from
        // `NVSDK_NGX_D3D12_CreateFeature` and every `HostList` referencing it
        // has finished and been reset or dropped.
        unsafe { NVSDK_NGX_D3D12_ReleaseFeature(raw as *mut c_void) };
    }
}

struct Dlss {
    params: *mut c_void,
    feature: Option<Arc<Feature>>,
}

// SAFETY: the SDK's objects are used under the mutex only.
unsafe impl Send for Dlss {}

/// `None` until tried; `Some(None)`: unavailable, never tried again.
static DLSS: Mutex<Option<Option<Dlss>>> = Mutex::new(None);

fn env_f(name: &str, default: f32) -> f32 {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn env_i(name: &str) -> Option<c_int> {
    std::env::var(name).ok().and_then(|v| v.trim().parse().ok())
}

fn env_flag(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|v| v.trim() == "1")
}

/// Parses `VICTORY_DLSS_PRESET` into an `NVSDK_NGX_DLSS_Hint_Render_Preset`
/// value (`J`=10, `K`=11 [DLSS 4 Transformer], `L`=12 [DLSS 4.5 UltraPerf],
/// `M`=13 [DLSS 4.5 Transformer], or an explicit positive integer).
fn dlss_preset() -> Option<(c_uint, String)> {
    let raw = std::env::var("VICTORY_DLSS_PRESET").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") || trimmed == "0" {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    let code = match upper.as_str() {
        "J" => 10,
        "K" => 11,
        "L" => 12,
        "M" => 13,
        _ => upper.parse::<c_uint>().ok().filter(|&v| v > 0)?,
    };
    Some((code, upper))
}

fn auto_quality(render_w: u32, out_w: u32) -> c_int {
    if out_w == 0 {
        return 2;
    }
    let ratio = render_w as f32 / out_w as f32;
    if ratio >= 0.95 {
        5 // DLAA
    } else if ratio >= 0.62 {
        2 // MaxQuality (~0.67)
    } else if ratio >= 0.54 {
        1 // Balanced (~0.58)
    } else if ratio >= 0.45 {
        0 // MaxPerf (~0.50)
    } else {
        3 // UltraPerformance (~0.33)
    }
}

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain([0]).collect()
}

fn init(device: &ID3D12Device) -> Option<Dlss> {
    let dir = std::env::var_os("VICTORY_DLSS_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok()?.parent().map(|p| p.to_path_buf()))?;
    let data = std::env::var_os("VICTORY_DATA_DIR")
        .map(|d| std::path::PathBuf::from(d).join("ngx"))
        .unwrap_or_else(|| std::env::temp_dir().join("victory-ngx"));
    let _ = std::fs::create_dir_all(&data);
    let dir_w = wide(dir.as_os_str());
    let data_w = wide(data.as_os_str());
    let paths = [dir_w.as_ptr()];
    let info = CommonInfo {
        paths: PathList { path: paths.as_ptr(), length: 1 },
        internal: std::ptr::null(),
        logging: Logging { callback: std::ptr::null(), minimum: 0, disable_other_sinks: false },
    };
    let project = CString::new("8436a65b-960d-40cd-ad41-6cec2c4a8c9f").ok()?;
    let version = CString::new("1.0").ok()?;
    // SAFETY: live strings and device; the SDK copies what it keeps.
    let r = unsafe {
        NVSDK_NGX_D3D12_Init_with_ProjectID(
            project.as_ptr(),
            ENGINE_CUSTOM,
            version.as_ptr(),
            data_w.as_ptr(),
            device.as_raw(),
            &info,
            VERSION_API,
        )
    };
    if r != SUCCESS {
        tracing::warn!(result = format_args!("{r:#x}"), dir = %dir.display(), "DLSS: the SDK did not start; the upscale stays a stretch");
        return None;
    }
    let mut params = std::ptr::null_mut();
    // SAFETY: an out pointer.
    let r = unsafe { NVSDK_NGX_D3D12_GetCapabilityParameters(&mut params) };
    if r != SUCCESS || params.is_null() {
        tracing::warn!(result = format_args!("{r:#x}"), "DLSS: no capability parameters");
        return None;
    }
    let mut available = 0;
    // SAFETY: live parameters.
    unsafe { NVSDK_NGX_Parameter_GetI(params, name!("SuperSampling.Available"), &mut available) };
    if available == 0 {
        tracing::warn!("DLSS: super sampling is not available on this card or driver");
        return None;
    }
    tracing::info!(dir = %dir.display(), "DLSS: started; the title's upscale is DLSS (VICTORY_UPSCALE=dlss)");
    Some(Dlss { params, feature: None })
}

/// Records DLSS over `inputs` into `list`; false if it could not (the caller
/// stretches instead). The list's descriptor heaps and root state are the
/// SDK's afterwards: the caller replays the title's.
pub(crate) fn record(device: &ID3D12Device, list: &crate::host::HostList, inputs: &Inputs<'_>) -> bool {
    let mut guard = DLSS.lock().unwrap_or_else(|e| e.into_inner());
    let dlss = match guard.get_or_insert_with(|| init(device)) {
        Some(d) => d,
        None => return false,
    };
    drain_retired();
    // SAFETY: live resources.
    let (out, colour) = unsafe { (inputs.output.GetDesc(), inputs.colour.GetDesc()) };
    let render = [
        if inputs.render[0] == 0 { colour.Width as u32 } else { inputs.render[0] },
        if inputs.render[1] == 0 { colour.Height } else { inputs.render[1] },
    ];
    // The feature's input size is the largest the colour texture holds, so
    // that a dynamic render size inside it needs no new feature.
    let (in_w, in_h) = (colour.Width as u32, colour.Height);
    let hdr = env_flag("VICTORY_DLSS_HDR").unwrap_or(matches!(colour.Format.0, 2 | 10 | 26 | 24 | 9));
    let inverted = env_flag("VICTORY_DLSS_DEPTH_INVERTED").unwrap_or(inputs.near > inputs.far);
    // FFX bit 2 = ENABLE_MOTION_VECTORS_JITTER_CANCELLATION (1 = jittered MVs, 0 = unjittered).
    let mv_jittered = env_flag("VICTORY_DLSS_MV_JITTERED").unwrap_or((inputs.create_flags & (1 << 2)) != 0);
    let mut flags = FLAG_MV_LOW_RES;
    if mv_jittered {
        flags |= FLAG_MV_JITTERED;
    }
    if hdr {
        flags |= FLAG_HDR;
    }
    if inverted {
        flags |= FLAG_DEPTH_INVERTED;
    }
    if inputs.exposure.is_none() {
        flags |= FLAG_AUTO_EXPOSURE;
    }
    let key = [in_w, in_h, out.Width as u32, out.Height, flags as u32];
    let p = dlss.params;
    if dlss.feature.as_ref().is_none_or(|f| f.key != key) {
        // Dropping the cached `Arc` queues the handle for NGX release only
        // once every `HostList` that recorded it has finished and reset.
        drop(dlss.feature.take());
        drain_retired();
        let quality = env_i("VICTORY_DLSS_QUALITY").unwrap_or_else(|| auto_quality(render[0], out.Width as u32));
        let preset = dlss_preset();
        let mut handle = std::ptr::null_mut();
        // SAFETY: live parameters and an open list.
        let r = unsafe {
            NVSDK_NGX_Parameter_SetUI(p, name!("CreationNodeMask"), 1);
            NVSDK_NGX_Parameter_SetUI(p, name!("VisibilityNodeMask"), 1);
            NVSDK_NGX_Parameter_SetUI(p, name!("Width"), in_w);
            NVSDK_NGX_Parameter_SetUI(p, name!("Height"), in_h);
            NVSDK_NGX_Parameter_SetUI(p, name!("OutWidth"), out.Width as u32);
            NVSDK_NGX_Parameter_SetUI(p, name!("OutHeight"), out.Height);
            NVSDK_NGX_Parameter_SetI(p, name!("PerfQualityValue"), quality);
            NVSDK_NGX_Parameter_SetI(p, name!("DLSS.Feature.Create.Flags"), flags);
            NVSDK_NGX_Parameter_SetI(p, name!("DLSS.Enable.Output.Subrects"), 0);
            if let Some((code, _)) = &preset {
                NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Hint.Render.Preset.DLAA"), *code);
                NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Hint.Render.Preset.Quality"), *code);
                NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Hint.Render.Preset.Balanced"), *code);
                NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Hint.Render.Preset.Performance"), *code);
                NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Hint.Render.Preset.UltraPerformance"), *code);
                NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Hint.Render.Preset.UltraQuality"), *code);
            }
            NVSDK_NGX_D3D12_CreateFeature(list.list.as_raw(), FEATURE_SUPER_SAMPLING, p, &mut handle)
        };
        if r != SUCCESS || handle.is_null() {
            tracing::warn!(result = format_args!("{r:#x}"), ?key, "DLSS: no feature for these sizes; this upscale stays a stretch");
            return false;
        }
        let preset_label = preset.as_ref().map_or("default", |(_, s)| s.as_str());
        tracing::info!(
            input = ?(in_w, in_h),
            render = ?render,
            output = ?(out.Width, out.Height),
            hdr,
            depth_inverted = inverted,
            mv_jittered,
            auto_exposure = inputs.exposure.is_none(),
            quality,
            preset = preset_label,
            colour_format = colour.Format.0,
            "DLSS: feature made"
        );
        dlss.feature = Some(Arc::new(Feature { handle, key }));
    }
    let Some(feature) = dlss.feature.as_ref() else { return false };
    list.keep_any(Box::new(Arc::clone(feature)));
    let mv = env_f("VICTORY_DLSS_MV_SIGN", 1.0);
    let jit = env_f("VICTORY_DLSS_JITTER_SIGN", 1.0);
    let raw = |r: Option<&ID3D12Resource>| r.map_or(std::ptr::null_mut(), |r| r.as_raw());
    let scale = |s: f32| if s == 0.0 { 1.0 } else { s };
    // SAFETY: live parameters, resources and feature; an open list.
    let r = unsafe {
        NVSDK_NGX_Parameter_SetD3d12Resource(p, name!("Color"), inputs.colour.as_raw());
        NVSDK_NGX_Parameter_SetD3d12Resource(p, name!("Output"), inputs.output.as_raw());
        NVSDK_NGX_Parameter_SetD3d12Resource(p, name!("Depth"), raw(inputs.depth));
        NVSDK_NGX_Parameter_SetD3d12Resource(p, name!("MotionVectors"), raw(inputs.motion));
        NVSDK_NGX_Parameter_SetD3d12Resource(p, name!("ExposureTexture"), raw(inputs.exposure));
        NVSDK_NGX_Parameter_SetF(p, name!("Jitter.Offset.X"), jit * inputs.jitter[0]);
        NVSDK_NGX_Parameter_SetF(p, name!("Jitter.Offset.Y"), jit * inputs.jitter[1]);
        NVSDK_NGX_Parameter_SetF(p, name!("MV.Scale.X"), mv * scale(inputs.motion_scale[0]));
        NVSDK_NGX_Parameter_SetF(p, name!("MV.Scale.Y"), mv * scale(inputs.motion_scale[1]));
        NVSDK_NGX_Parameter_SetI(p, name!("Reset"), inputs.reset as c_int);
        NVSDK_NGX_Parameter_SetF(p, name!("Sharpness"), env_f("VICTORY_DLSS_SHARPNESS", 0.0));
        NVSDK_NGX_Parameter_SetF(p, name!("DLSS.Pre.Exposure"), if inputs.pre_exposure > 0.0 { inputs.pre_exposure } else { 1.0 });
        NVSDK_NGX_Parameter_SetF(p, name!("FrameTimeDeltaInMsec"), inputs.frame_ms);
        NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Render.Subrect.Dimensions.Width"), render[0].min(in_w));
        NVSDK_NGX_Parameter_SetUI(p, name!("DLSS.Render.Subrect.Dimensions.Height"), render[1].min(in_h));
        NVSDK_NGX_D3D12_EvaluateFeature_C(list.list.as_raw(), feature.handle, p, std::ptr::null())
    };
    if r != SUCCESS {
        static TOLD: std::sync::Once = std::sync::Once::new();
        TOLD.call_once(|| tracing::warn!(result = format_args!("{r:#x}"), "DLSS: an evaluation failed; that upscale is a stretch"));
        return false;
    }
    true
}
