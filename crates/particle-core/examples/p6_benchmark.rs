use particle_core::{
    ParticleConfig, RenderWorkspace, TrailMode, build_funnel, render_batch, render_batch_cached,
};
use std::{
    hint::black_box,
    mem::size_of_val,
    time::{Duration, Instant},
};

const FRAMES: usize = 60;

fn config_for(live_particles: usize, trail: bool) -> ParticleConfig {
    let mut config = ParticleConfig {
        frequency: live_particles as f64,
        lifetime: 10.0,
        spread_degrees: 180.0,
        ..Default::default()
    };
    if trail {
        config.p3.trail.mode = TrailMode::Afterimage;
        config.p3.trail.length = 0.5;
        config.p3.trail.samples = 4;
        config.p3.trail.opacity = 0.5;
    }
    config
}

fn percentile(samples: &mut [Duration], percentile: usize) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percentile / 100]
}

fn measure(name: &str, particles: usize, trail: bool, funnel: bool, cached: bool) {
    let config = config_for(particles, trail);
    let mut workspace = RenderWorkspace::default();
    for frame in 0..3 {
        let time = 10.0 + frame as f64 / 60.0;
        let batch = if cached {
            render_batch_cached(&config, time, &mut workspace)
        } else {
            render_batch(&config, time)
        };
        black_box(batch);
    }

    let mut samples = Vec::with_capacity(FRAMES);
    let mut output_items = 0usize;
    let mut output_bytes = 0usize;
    for frame in 0..FRAMES {
        let time = 10.0 + frame as f64 / 60.0;
        let started = Instant::now();
        let batch = if cached {
            render_batch_cached(&config, time, &mut workspace)
        } else {
            render_batch(&config, time)
        };
        let children = if funnel {
            build_funnel(&batch.particles, time, 4, 30.0, 0.5, 90.0, 10_000)
        } else {
            Vec::new()
        };
        samples.push(started.elapsed());
        output_items = batch.particles.len()
            + batch.trail_images.len()
            + batch.trail_segments.len()
            + children.len();
        output_bytes = size_of_val(batch.particles.as_slice())
            + size_of_val(batch.trail_images.as_slice())
            + size_of_val(batch.trail_segments.as_slice())
            + size_of_val(children.as_slice());
        black_box((batch, children));
    }
    let median = percentile(&mut samples.clone(), 50).as_secs_f64() * 1000.0;
    let p95 = percentile(&mut samples, 95).as_secs_f64() * 1000.0;
    println!(
        "| {name} | {} | {particles} | {output_items} | {median:.3} | {p95:.3} | {:.2} |",
        if cached { "warm cache" } else { "stateless" },
        output_bytes as f64 / (1024.0 * 1024.0)
    );
}

fn main() {
    println!("particle2r P6 core benchmark (release, {FRAMES} frames)");
    println!("| case | mode | live particles | output items | median ms | p95 ms | output MiB |");
    println!("|---|---|---:|---:|---:|---:|---:|");
    for cached in [false, true] {
        measure("basic", 1_000, false, false, cached);
        measure("basic", 10_000, false, false, cached);
        measure("trail x4", 1_000, true, false, cached);
        measure("trail x4", 10_000, true, false, cached);
        measure("funnel x4", 1_000, false, true, cached);
        measure("funnel x4", 10_000, false, true, cached);
    }
    println!();
    println!("Framebuffer RGBA estimates: 1920x1080 = 7.91 MiB, 3840x2160 = 31.64 MiB.");
    println!(
        "Host GPU draw time is measured separately in AviUtl2; this executable measures CPU calculation only."
    );
}
