use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Duration,
};

#[derive(Default)]
struct Stats {
    frames: u64,
    calculation_ms: f64,
    draw_ms: f64,
    total_ms: f64,
    max_total_ms: f64,
}

pub(super) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("PARTICLE2R_PROFILE")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

pub(super) fn record(
    resolution: [u32; 2],
    particles: usize,
    output_items: usize,
    calculation: Duration,
    total: Duration,
) {
    type Key = (u32, u32, usize, usize);
    static STATS: OnceLock<Mutex<HashMap<Key, Stats>>> = OnceLock::new();
    let Ok(mut all) = STATS.get_or_init(|| Mutex::new(HashMap::new())).lock() else {
        return;
    };
    let stats = all
        .entry((resolution[0], resolution[1], particles, output_items))
        .or_default();
    let calculation_ms = calculation.as_secs_f64() * 1000.0;
    let total_ms = total.as_secs_f64() * 1000.0;
    let draw_ms = (total_ms - calculation_ms).max(0.0);
    stats.frames += 1;
    stats.calculation_ms += calculation_ms;
    stats.draw_ms += draw_ms;
    stats.total_ms += total_ms;
    stats.max_total_ms = stats.max_total_ms.max(total_ms);
    if stats.frames == 1 || stats.frames % 60 == 0 {
        let count = stats.frames as f64;
        let _ = aviutl2::logger::write_info_log(&format!(
            "particle2r-profile {}x{} particles={} items={} frames={} calc_avg_ms={:.3} draw_avg_ms={:.3} total_avg_ms={:.3} total_max_ms={:.3}",
            resolution[0],
            resolution[1],
            particles,
            output_items,
            stats.frames,
            stats.calculation_ms / count,
            stats.draw_ms / count,
            stats.total_ms / count,
            stats.max_total_ms,
        ));
    }
}
