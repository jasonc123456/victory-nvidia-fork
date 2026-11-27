//! `VICTORY_SKIP_WC_BACK=1` (off by default): what shaders wrote to a card
//! copy of title memory that the title reserved write-combining is not
//! fetched back into title memory.
//!
//! From fxceless/victory-fork PR 868 (Chipsraine, with @iGoldenNinja's
//! reservation check, f9c43fa): the part that changes what is fetched; the
//! census logging of that PR (`VICTORY_CENSUS`) is not brought over. Its
//! census found 97-99 % of the ~100 MB a frame fetched back came from
//! memory the title reserved write-combining, memory the console's CPU
//! writes but does not read back; testers on NVIDIA cards measured 11-22 %
//! shorter frames (copy-back 65-100 ms -> about 20) and no device loss.
//!
//! shortcut: it assumes the title's CPU never reads write-combining memory
//! the card wrote, and never writes part of a 64 KiB piece the card wrote
//! (the next upload would send the stale bytes there). A correct form would
//! keep such pieces on the card and fetch one on the CPU's first touch.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// The console's write-combining protection bit.
const PAGE_WRITECOMBINE: u32 = 0x400;

/// A reservation of `XMemVirtualAlloc`, as the title asked for it.
struct Reservation {
    len: usize,
    protect: u32,
}

static RESERVATIONS: Mutex<BTreeMap<usize, Reservation>> = Mutex::new(BTreeMap::new());

/// The runtime reserved `[at, at + len)` for the title (`XMemVirtualAlloc`
/// at an address of its choice), with the title's own console `protect`
/// (before the runtime cleans it of the write-combining bit).
pub fn title_memory_reserved(at: usize, len: usize, protect: u32) {
    if !skip_wc_back() || at == 0 || len == 0 {
        return;
    }
    RESERVATIONS.lock().unwrap_or_else(|e| e.into_inner()).insert(at, Reservation { len, protect });
}

/// Whether the switch is on (`VICTORY_SKIP_WC_BACK=1`).
pub(crate) fn skip_wc_back() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        let on = std::env::var_os("VICTORY_SKIP_WC_BACK").is_some_and(|v| v == "1");
        if on {
            tracing::info!(
                "VICTORY_SKIP_WC_BACK=1: shader writes to title memory reserved write-combining are not copied back (fxceless PR 868)"
            );
        }
        on
    })
}

/// Every range lies wholly inside a single write-combining reservation. A
/// card copy (an aligned loose window above all) can cover several.
fn all_wc_in(reservations: &BTreeMap<usize, Reservation>, ranges: &[(usize, usize)]) -> bool {
    !ranges.is_empty()
        && ranges.iter().all(|&(from, to)| {
            // Seeking from the exclusive end also finds a reservation that
            // begins inside the range. One that ends at `to` fits.
            from < to
                && reservations.range(..to).next_back().is_some_and(|(&base, r)| {
                    base <= from && r.protect & PAGE_WRITECOMBINE != 0 && base.checked_add(r.len).is_some_and(|end| to <= end)
                })
        })
}

/// Pieces left on the card, and pieces fetched after all because the title
/// writes their pages or their declaration is small: count and bytes each.
static SKIPPED: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];

/// Declared ranges smaller than this are always fetched
/// (`VICTORY_SKIP_WC_MIN_KB`, 256; 0: none). On the first run with the
/// switch the whole picture flashed every frame: the title's CPU reads some
/// small results the card writes into write-combining memory (a guess:
/// the exposure the frame is shown at), and the bulk of what was fetched
/// back lies in large ranges.
fn min_declared() -> usize {
    static KB: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *KB.get_or_init(|| {
        std::env::var("VICTORY_SKIP_WC_MIN_KB").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(256)
    }) << 10
}

/// Whether pages the title writes are fetched (`VICTORY_SKIP_WC_TITLE_PAGES`,
/// on; 0: not). A page sent up to the card twice or more holds bytes the
/// title's CPU writes: sent up whole, it would put the title's stale copy of
/// what the card wrote beside them over the card's.
fn title_pages() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_SKIP_WC_TITLE_PAGES").is_none_or(|v| v != "0"))
}

/// The granularity of [`SENT`]: 4 KiB, the write watch's.
const SENT_PAGE: usize = 12;

/// Pages of write-combining memory sent up to the card's copies, by title
/// page number: how often (to 2). The first send is the copy being filled.
static SENT: Mutex<Option<std::collections::HashMap<usize, u8>>> = Mutex::new(None);

/// A submission sends up the ranges `ranges` (offsets) of the copy of title
/// memory starting at `start`.
pub(crate) fn sent_up(start: usize, ranges: &[(usize, usize)]) {
    if !skip_wc_back() || !title_pages() || ranges.is_empty() {
        return;
    }
    let mut sent = SENT.lock().unwrap_or_else(|e| e.into_inner());
    let sent = sent.get_or_insert_with(Default::default);
    for &(a, b) in ranges {
        if a >= b {
            continue;
        }
        for page in (start + a) >> SENT_PAGE..=(start + b - 1) >> SENT_PAGE {
            let n = sent.entry(page).or_insert(0);
            *n = (*n + 1).min(2);
        }
    }
}

/// Whether a page of `[from, to)` was sent up more than once.
fn title_writes(from: usize, to: usize) -> bool {
    let sent = SENT.lock().unwrap_or_else(|e| e.into_inner());
    let Some(sent) = sent.as_ref() else { return false };
    (from >> SENT_PAGE..=(to - 1) >> SENT_PAGE).any(|page| sent.get(&page).is_some_and(|&n| n >= 2))
}

/// Whether the piece `piece` (title addresses) of the declared range
/// `declared` stays on the card. A piece is clipped to its declaration; a
/// declaration that is not all write-combining keeps every piece, so the
/// normal fetch covers it in full.
pub(crate) fn skip_back(declared: (usize, usize), piece: (usize, usize)) -> bool {
    if !skip_wc_back() || piece.0 >= piece.1 {
        return false;
    }
    if !all_wc_in(&RESERVATIONS.lock().unwrap_or_else(|e| e.into_inner()), &[declared]) {
        return false;
    }
    let bytes = (piece.1 - piece.0) as u64;
    let kept = if declared.1 - declared.0 < min_declared() {
        Some(2)
    } else if title_pages() && title_writes(piece.0, piece.1) {
        Some(4)
    } else {
        None
    };
    if let Some(at) = kept {
        SKIPPED[at].fetch_add(1, Ordering::Relaxed);
        SKIPPED[at + 1].fetch_add(bytes, Ordering::Relaxed);
        return false;
    }
    let n = SKIPPED[0].fetch_add(1, Ordering::Relaxed) + 1;
    let b = SKIPPED[1].fetch_add(bytes, Ordering::Relaxed) + bytes;
    if n.is_power_of_two() {
        tracing::info!(
            "VICTORY_SKIP_WC_BACK: {n} pieces ({} MB) of write-combining memory left on the card so far; fetched after all: {} pieces ({} MB) of small declarations, {} pieces ({} MB) whose pages the title writes",
            b >> 20,
            SKIPPED[2].load(Ordering::Relaxed),
            SKIPPED[3].load(Ordering::Relaxed) >> 20,
            SKIPPED[4].load(Ordering::Relaxed),
            SKIPPED[5].load(Ordering::Relaxed) >> 20,
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{Reservation, all_wc_in};
    use std::collections::BTreeMap;

    fn reservation(len: usize, protect: u32) -> Reservation {
        Reservation { len, protect }
    }

    #[test]
    fn a_write_in_the_ordinary_half_of_a_card_window_still_comes_back() {
        const MIB: usize = 1 << 20;
        let window = 8 * MIB;
        let middle = window + 2 * MIB;
        let map = BTreeMap::from([(window, reservation(2 * MIB, 0x404)), (middle, reservation(2 * MIB, 0x04))]);
        let wc = (middle - 65536, middle);
        let ordinary = (middle, middle + 65536);
        assert!(all_wc_in(&map, &[wc]));
        assert!(!all_wc_in(&map, &[ordinary]));
        assert!(!all_wc_in(&map, &[wc, ordinary]));
        assert!(!all_wc_in(&map, &[(wc.0, ordinary.1)]));
    }

    #[test]
    fn a_declaration_must_fit_one_known_reservation() {
        let map = BTreeMap::from([
            (0x10000, reservation(0x10000, 0x404)),
            (0x20000, reservation(0x10000, 0x404)),
            (usize::MAX - 8, reservation(16, 0x404)),
        ]);
        assert!(all_wc_in(&map, &[(0x10000, 0x20000), (0x20000, 0x30000)]));
        assert!(!all_wc_in(&map, &[(0x1ffff, 0x20001)]));
        assert!(!all_wc_in(&map, &[(0x30000, 0x30001)]));
        assert!(!all_wc_in(&map, &[(0x10000, 0x10000)]));
        assert!(!all_wc_in(&map, &[(usize::MAX - 8, 1)]));
        assert!(!all_wc_in(&map, &[(usize::MAX - 8, usize::MAX)]));
    }
}
