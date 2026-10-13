//! Cumulative allocation/transfer traffic, NOT live category residency.
//!
//! Works on every platform (desktop users can run it too). Categories are plain
//! Legacy category call sites do not all pair add/sub, so their totals must
//! never be described as live bytes. FPV_RETAINED_BYTES is a separate ownership
//! gauge paired with PayloadRetention::Drop; it overlaps active assets and
//! must not be added to process heap/footprint.

use std::sync::atomic::{AtomicU64, Ordering};

pub static FPV_RETAINED_BYTES: AtomicU64 = AtomicU64::new(0);
pub static IMAGE_MOVED_BYTES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cat {
    /// RGBA8/BC texels held in `Image.data` that `Assets<Image>` keeps in the main world.
    ImageCpu,
    /// Bytes of images handed to the GPU (`Assets<Image>::add`).
    ImageToGpu,
    /// Extra copies of image data made while preparing (clone of an already-built image).
    ImageDupCopy,
    /// Mesh vertex and index data kept in the main world (`MAIN_WORLD` usage).
    MeshCpu,
    /// Lightmap pages (six images each).
    Lightmap,
    /// Decoded audio kept resident.
    Audio,
    /// Everything else a caller wants to tag.
    Other,
}

const COUNT: usize = 7;

struct Slot {
    live: AtomicU64,
    peak: AtomicU64,
    total: AtomicU64,
    events: AtomicU64,
}

impl Slot {
    const fn new() -> Self {
        Self {
            live: AtomicU64::new(0),
            peak: AtomicU64::new(0),
            total: AtomicU64::new(0),
            events: AtomicU64::new(0),
        }
    }
}

static SLOTS: [Slot; COUNT] = [const { Slot::new() }; COUNT];

const fn name(cat: Cat) -> &'static str {
    match cat {
        Cat::ImageCpu => "img_cpu",
        Cat::ImageToGpu => "img_to_gpu",
        Cat::ImageDupCopy => "img_dup",
        Cat::MeshCpu => "mesh_cpu",
        Cat::Lightmap => "lightmap",
        Cat::Audio => "audio",
        Cat::Other => "other",
    }
}

const ALL: [Cat; COUNT] = [
    Cat::ImageCpu,
    Cat::ImageToGpu,
    Cat::ImageDupCopy,
    Cat::MeshCpu,
    Cat::Lightmap,
    Cat::Audio,
    Cat::Other,
];

fn slot(cat: Cat) -> &'static Slot {
    &SLOTS[cat as usize]
}

/// Record `bytes` newly held in `cat`.
pub fn add(cat: Cat, bytes: u64) {
    let s = slot(cat);
    let live = s.live.fetch_add(bytes, Ordering::Relaxed) + bytes;
    s.total.fetch_add(bytes, Ordering::Relaxed);
    s.events.fetch_add(1, Ordering::Relaxed);
    s.peak.fetch_max(live, Ordering::Relaxed);
}

/// Record `bytes` released from `cat`.
pub fn sub(cat: Cat, bytes: u64) {
    let s = slot(cat);
    let _ = s
        .live
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
            Some(v.saturating_sub(bytes))
        });
}

pub fn live(cat: Cat) -> u64 {
    slot(cat).live.load(Ordering::Relaxed)
}

const MB: u64 = 1024 * 1024;

/// Cumulative traffic. These numbers are not current CPU/GPU allocations.
pub fn snapshot_line() -> String {
    let mut parts = Vec::with_capacity(COUNT);
    for cat in ALL {
        let s = slot(cat);
        let total = s.total.load(Ordering::Relaxed) / MB;
        let events = s.events.load(Ordering::Relaxed);
        if events == 0 {
            continue;
        }
        parts.push(format!("{} total {total}MiB ({events}x)", name(cat)));
    }
    if parts.is_empty() {
        "no tracked categories yet".to_owned()
    } else {
        format!("cumulative traffic (not live): {}", parts.join(" | "))
    }
}

/// Full multi-line report, for the end of a load or on request.
pub fn report() -> String {
    let mut out = String::from("category traffic (cumulative; not live CPU/GPU residency)\n");
    for cat in ALL {
        let s = slot(cat);
        out.push_str(&format!(
            "  {:<11} total {:>6} MiB  events {}\n",
            name(cat),
            s.total.load(Ordering::Relaxed) / MB,
            s.events.load(Ordering::Relaxed),
        ));
    }
    out
}
