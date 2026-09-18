//! Host independent, deterministic particle sampling for the Particle (R) port.

use std::f64::consts::{PI, TAU};

mod p3;
pub use p3::{
    Arrival, Boundary, Convergence, Dispersion, Modulation, Orbit, OrbitPlane, P3Config, TimeWarp,
    Trail, TrailMode, Variation, Wave, WindCurve,
};

const MAX_PARTICLES: usize = 10_000;
const MAX_TRAIL_ITEMS: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitterShape {
    Point,
    Line,
    Box,
    Sphere,
}

#[derive(Clone, Copy, Debug)]
pub struct ParticleConfig {
    /// Legacy output speed: distance travelled in one second.
    pub speed: f64,
    /// Legacy frequency track: 100 means 10 emission events per second.
    pub frequency: f64,
    pub direction_degrees: f64,
    pub spread_degrees: f64,
    pub direction_z_degrees: f64,
    pub spread_z_degrees: f64,
    pub rotation_z_degrees_per_second: f64,
    pub simultaneous: u32,
    pub lifetime: f64,
    pub start_time: f64,
    pub gravity_x: f64,
    pub gravity_y: f64,
    pub gravity_z: f64,
    pub alpha_start: f64,
    pub alpha_end: f64,
    pub zoom_start: f64,
    pub zoom_end: f64,
    pub shape: EmitterShape,
    pub extent_x: f64,
    pub extent_y: f64,
    pub extent_z: f64,
    pub seed: i32,
    pub layer: u32,
    pub p3: P3Config,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            speed: 100.0,
            frequency: 100.0,
            direction_degrees: 0.0,
            spread_degrees: 60.0,
            direction_z_degrees: 0.0,
            spread_z_degrees: 0.0,
            rotation_z_degrees_per_second: 60.0,
            simultaneous: 1,
            lifetime: 3.0,
            start_time: 0.0,
            gravity_x: 0.0,
            gravity_y: 0.0,
            gravity_z: 0.0,
            alpha_start: 1.0,
            alpha_end: 0.5,
            zoom_start: 1.0,
            zoom_end: 1.0,
            shape: EmitterShape::Point,
            extent_x: 100.0,
            extent_y: 100.0,
            extent_z: 100.0,
            seed: 0,
            layer: 0,
            p3: P3Config::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSample {
    pub id: u64,
    pub birth_time: f64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rx: f32,
    pub ry: f32,
    pub rz: f32,
    pub scale: f32,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailSegment {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub width: f32,
    pub alpha: f32,
}

#[derive(Default, Debug, PartialEq)]
pub struct RenderBatch {
    pub particles: Vec<ParticleSample>,
    pub trail_images: Vec<ParticleSample>,
    pub trail_segments: Vec<TrailSegment>,
}

struct ParticleSeed {
    id: u64,
    birth_time: f64,
    lifetime: f64,
    origin: [f64; 3],
    initial_velocity: [f64; 3],
    gravity: [f64; 3],
    dispersion_impulse: [f64; 3],
    initial_rotation_z: f64,
    rotation_speed_z: f64,
    alpha_start: f64,
    alpha_end: f64,
    zoom_start: f64,
    zoom_end: f64,
}

/// Low 32 bits of the seed mixture recovered from the original DLL.
pub fn legacy_seed(seed: i32, layer: u32) -> u32 {
    let r = seed as u32;
    let l = if seed < 0 { 0 } else { layer };
    0x05e3_0a6d_u32
        .wrapping_mul(l.wrapping_mul(l).wrapping_mul(l))
        .wrapping_add(0x0087_1e4b_u32.wrapping_mul(l).wrapping_mul(r))
        .wrapping_add(0x077a_5195_u32.wrapping_mul(r.wrapping_mul(r).wrapping_mul(r)))
        .wrapping_add(0x0001_81dd)
}

/// The MT19937 generator found in the original DLL. Random consumption per
/// emission is currently a new, seek stable convention, not a recovered trace.
pub struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    pub fn new(seed: u32) -> Self {
        let mut state = [0; 624];
        state[0] = seed;
        for i in 1..624 {
            let previous = state[i - 1];
            state[i] = 1_812_433_253_u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.index == 624 {
            for i in 0..624 {
                let x = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (x >> 1)
                    ^ if x & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut x = self.state[self.index];
        self.index += 1;
        x ^= x >> 11;
        x ^= (x << 7) & 0x9d2c_5680;
        x ^= (x << 15) & 0xefc6_0000;
        x ^= x >> 18;
        x
    }

    fn unit(&mut self) -> f64 {
        self.next_u32() as f64 / (u32::MAX as f64 + 1.0)
    }
}

/// Samples living particles from absolute object time. This function keeps the
/// P2 API; use [`render_batch`] to obtain the optional trail geometry.
pub fn sample(config: &ParticleConfig, time: f64) -> Vec<ParticleSample> {
    render_internal(config, time, false).particles
}

/// Builds particles and P3 trails without retaining state between frames.
pub fn render_batch(config: &ParticleConfig, time: f64) -> RenderBatch {
    render_internal(config, time, true)
}

fn render_internal(config: &ParticleConfig, time: f64, with_trails: bool) -> RenderBatch {
    let mut batch = RenderBatch::default();
    if !time.is_finite()
        || !config.lifetime.is_finite()
        || config.lifetime <= 0.0
        || !config.frequency.is_finite()
        || config.frequency <= 0.0
        || !config.start_time.is_finite()
        || time < config.start_time
        || config.simultaneous == 0
    {
        return batch;
    }

    let elapsed = time - config.start_time;
    let longest_lifetime =
        config.lifetime * (1.0 + config.p3.variation.lifetime_percent.abs() / 100.0).max(1.0);
    let trail_length = if with_trails && config.p3.trail.mode != TrailMode::Off {
        config.p3.trail.length.max(0.0)
    } else {
        0.0
    };
    let last_event = emission_phase(config, elapsed).floor() as u64;
    let first_event =
        emission_phase(config, (elapsed - longest_lifetime - trail_length).max(0.0)).floor() as u64;
    let max_events = (MAX_PARTICLES / config.simultaneous as usize).max(1) as u64;
    let first_event = first_event.max(last_event.saturating_sub(max_events - 1));
    batch.particles.reserve(
        MAX_PARTICLES.min(
            (last_event.saturating_sub(first_event) as usize + 1)
                .saturating_mul(config.simultaneous as usize),
        ),
    );

    for event in first_event..=last_event {
        let birth_time = config.start_time + birth_offset(config, event);
        let age = time - birth_time;
        if age < 0.0 || age > longest_lifetime + trail_length {
            continue;
        }
        for sibling in 0..config.simultaneous {
            if batch.particles.len() == MAX_PARTICLES
                && (!with_trails
                    || trail_length == 0.0
                    || batch.trail_images.len() + batch.trail_segments.len() >= MAX_TRAIL_ITEMS)
            {
                return batch;
            }
            let seed = particle_seed(config, event, sibling, birth_time);
            if batch.particles.len() < MAX_PARTICLES {
                if let Some(particle) = sample_seed(config, &seed, age) {
                    batch.particles.push(particle);
                }
            }
            if with_trails && trail_length > 0.0 && config.p3.trail.mode != TrailMode::Off {
                append_trail(config, &seed, age, &mut batch);
            }
        }
    }
    batch
}

fn emission_phase(config: &ParticleConfig, elapsed: f64) -> f64 {
    let rate = config.frequency / 10.0;
    let wave = config.p3.modulation.frequency;
    if wave.depth == 0.0 || wave.period <= 0.0 {
        return elapsed * rate;
    }
    let depth = wave.depth.clamp(-0.95, 0.95);
    let omega = TAU / wave.period;
    rate * (elapsed + depth * (1.0 - (omega * elapsed).cos()) / omega)
}

fn birth_offset(config: &ParticleConfig, event: u64) -> f64 {
    let rate = config.frequency / 10.0;
    let wave = config.p3.modulation.frequency;
    if event == 0 || wave.depth == 0.0 || wave.period <= 0.0 {
        return event as f64 / rate;
    }
    let depth = wave.depth.abs().min(0.95);
    let mut lower = event as f64 / (rate * (1.0 + depth));
    let mut upper = event as f64 / (rate * (1.0 - depth));
    for _ in 0..45 {
        let mid = (lower + upper) * 0.5;
        if emission_phase(config, mid) < event as f64 {
            lower = mid;
        } else {
            upper = mid;
        }
    }
    (lower + upper) * 0.5
}

fn particle_seed(
    config: &ParticleConfig,
    event: u64,
    sibling: u32,
    birth_time: f64,
) -> ParticleSeed {
    let id = event
        .saturating_mul(config.simultaneous as u64)
        .saturating_add(sibling as u64);
    let mut rng = Mt19937::new(
        legacy_seed(config.seed, config.layer)
            ^ (event as u32).wrapping_mul(0x9e37_79b9)
            ^ sibling.wrapping_mul(0x85eb_ca6b),
    );
    let (ox, oy, oz) = spawn_position(config, &mut rng);
    let angle =
        (config.direction_degrees + (rng.unit() * 2.0 - 1.0) * config.spread_degrees) * PI / 180.0;
    let elevation =
        (config.direction_z_degrees + (rng.unit() * 2.0 - 1.0) * config.spread_z_degrees) * PI
            / 180.0;
    let initial_rotation_z = rng.unit() * 360.0;
    let variation = config.p3.variation;
    let speed = config.speed * variation_factor(&mut rng, variation.speed_percent);
    let lifetime = config.lifetime * variation_factor(&mut rng, variation.lifetime_percent);
    let rotation_speed_z = config.rotation_z_degrees_per_second
        * variation_factor(&mut rng, variation.rotation_percent);
    let gravity_factor = variation_factor(&mut rng, variation.gravity_percent);
    let alpha_factor = variation_factor(&mut rng, variation.alpha_percent);
    let zoom_factor = variation_factor(&mut rng, variation.zoom_percent);
    let dispersion_impulse = if config.p3.dispersion.enabled {
        p3::dispersion_vector(
            config.p3.dispersion.impulse,
            config.p3.dispersion.xy_spread_degrees,
            config.p3.dispersion.z_spread_degrees,
            rng.unit(),
            rng.unit(),
        )
    } else {
        [0.0; 3]
    };
    let speed_xy = speed * elevation.cos();
    ParticleSeed {
        id,
        birth_time,
        lifetime,
        origin: [ox, oy, oz],
        initial_velocity: [
            angle.sin() * speed_xy,
            -angle.cos() * speed_xy,
            elevation.sin() * speed,
        ],
        gravity: [
            config.gravity_x * gravity_factor,
            config.gravity_y * gravity_factor,
            config.gravity_z * gravity_factor,
        ],
        dispersion_impulse,
        initial_rotation_z,
        rotation_speed_z,
        alpha_start: config.alpha_start * alpha_factor,
        alpha_end: config.alpha_end * alpha_factor,
        zoom_start: config.zoom_start * zoom_factor,
        zoom_end: config.zoom_end * zoom_factor,
    }
}

fn variation_factor(rng: &mut Mt19937, percent: f64) -> f64 {
    if percent == 0.0 {
        1.0
    } else {
        (1.0 + (rng.unit() * 2.0 - 1.0) * percent.abs() / 100.0).max(0.0)
    }
}

fn sample_seed(config: &ParticleConfig, seed: &ParticleSeed, age: f64) -> Option<ParticleSample> {
    if age < 0.0 || age >= seed.lifetime || seed.lifetime <= 0.0 {
        return None;
    }
    let p3 = &config.p3;
    let motion_age = p3.time_warp.motion_age(age);
    let pos = p3.position(
        seed.birth_time,
        age,
        seed.origin,
        seed.initial_velocity,
        seed.gravity,
        seed.dispersion_impulse,
    )?;
    let progress = (age / seed.lifetime).clamp(0.0, 1.0);
    let alpha = (lerp(seed.alpha_start, seed.alpha_end, progress)
        * (1.0 + p3.modulation.alpha.value(motion_age)))
    .clamp(0.0, 1.0);
    let scale = (lerp(seed.zoom_start, seed.zoom_end, progress)
        * (1.0 + p3.modulation.zoom.value(motion_age)))
    .max(0.0);
    let rx = 0.5 * p3.rotation_acceleration[0] * motion_age * motion_age;
    let ry = 0.5 * p3.rotation_acceleration[1] * motion_age * motion_age;
    let rz = seed.initial_rotation_z
        + seed.rotation_speed_z * motion_age
        + 0.5 * p3.rotation_acceleration[2] * motion_age * motion_age;
    if [pos[0], pos[1], pos[2], rx, ry, rz, alpha, scale]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= f32::MAX as f64)
    {
        Some(ParticleSample {
            id: seed.id,
            birth_time: seed.birth_time,
            x: pos[0] as f32,
            y: pos[1] as f32,
            z: pos[2] as f32,
            rx: rx as f32,
            ry: ry as f32,
            rz: rz as f32,
            scale: scale as f32,
            alpha: alpha as f32,
        })
    } else {
        None
    }
}

fn append_trail(config: &ParticleConfig, seed: &ParticleSeed, age: f64, batch: &mut RenderBatch) {
    let trail = config.p3.trail;
    if age < 0.0 || age > seed.lifetime + trail.length || trail.samples == 0 {
        return;
    }
    let count = trail.samples.min(16);
    let mut newer = sample_seed(config, seed, age.min((seed.lifetime - 1e-9).max(0.0)));
    for index in 1..=count {
        if batch.trail_images.len() + batch.trail_segments.len() >= MAX_TRAIL_ITEMS {
            break;
        }
        let previous_age = age - index as f64 * trail.length / count as f64;
        let Some(mut previous) = sample_seed(config, seed, previous_age) else {
            continue;
        };
        let fade = 1.0 - index as f64 / (count as f64 + 1.0);
        let linger = if age > seed.lifetime {
            (1.0 - (age - seed.lifetime) / trail.length).clamp(0.0, 1.0)
        } else {
            1.0
        };
        previous.alpha *= (trail.opacity * fade * linger) as f32;
        if previous.alpha <= 0.0 {
            continue;
        }
        match trail.mode {
            TrailMode::Off => {}
            TrailMode::Afterimage | TrailMode::Points => {
                previous.scale *= if trail.mode == TrailMode::Points {
                    trail.scale as f32
                } else {
                    (1.0 - (1.0 - trail.scale) * index as f64 / count as f64) as f32
                };
                batch.trail_images.push(previous);
            }
            TrailMode::Ribbon => {
                if let Some(front) = newer {
                    batch.trail_segments.push(TrailSegment {
                        from: [front.x, front.y, front.z],
                        to: [previous.x, previous.y, previous.z],
                        width: trail.width as f32,
                        alpha: previous.alpha,
                    });
                }
            }
        }
        newer = Some(previous);
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn spawn_position(c: &ParticleConfig, rng: &mut Mt19937) -> (f64, f64, f64) {
    match c.shape {
        EmitterShape::Point => (0.0, 0.0, 0.0),
        EmitterShape::Line => ((rng.unit() - 0.5) * c.extent_x, 0.0, 0.0),
        EmitterShape::Box => (
            (rng.unit() - 0.5) * c.extent_x,
            (rng.unit() - 0.5) * c.extent_y,
            (rng.unit() - 0.5) * c.extent_z,
        ),
        EmitterShape::Sphere => {
            let z = rng.unit() * 2.0 - 1.0;
            let a = rng.unit() * TAU;
            let radial = (1.0 - z * z).sqrt();
            let radius = c.extent_x * 0.5;
            (
                radius * radial * a.cos(),
                radius * radial * a.sin(),
                radius * z,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mt_matches_reference_sequence() {
        let mut rng = Mt19937::new(5489);
        assert_eq!(
            (0..5).map(|_| rng.next_u32()).collect::<Vec<_>>(),
            [
                3_499_211_612,
                581_869_302,
                3_890_346_734,
                3_586_334_585,
                545_404_204
            ]
        );
    }

    #[test]
    fn frame_order_does_not_change_output() {
        let c = ParticleConfig::default();
        let before = sample(&c, 1.25);
        let _ = sample(&c, 9.0);
        assert_eq!(before, sample(&c, 1.25));
    }

    #[test]
    fn default_frequency_is_ten_events_per_second() {
        let c = ParticleConfig::default();
        assert_eq!(sample(&c, 0.0).len(), 1);
        assert_eq!(sample(&c, 1.0).len(), 11);
    }

    #[test]
    fn lifetime_and_limits_are_enforced() {
        let c = ParticleConfig {
            lifetime: 0.5,
            simultaneous: 10_000,
            ..Default::default()
        };
        assert_eq!(sample(&c, 0.0).len(), 10_000);
        assert_eq!(sample(&c, 1.0).len(), 10_000);
        assert!(sample(&ParticleConfig { lifetime: 0.0, ..c }, 1.0).is_empty());
    }

    #[test]
    fn rotation_uses_particle_age() {
        let c = ParticleConfig {
            frequency: 10.0,
            ..Default::default()
        };
        let initial = sample(&c, 0.0)[0];
        let after_one_second = sample(&c, 1.0)[0];
        assert_eq!(initial.id, after_one_second.id);
        assert!((after_one_second.rz - initial.rz - 60.0).abs() < 0.0001);
    }

    #[test]
    fn linear_wind_curve_accelerates_from_object_time() {
        let mut c = ParticleConfig {
            speed: 0.0,
            frequency: 10.0,
            ..Default::default()
        };
        c.p3.wind.to = [2.0, 0.0, 0.0];
        c.p3.wind.duration = 2.0;
        let first = sample(&c, 2.0).into_iter().find(|p| p.id == 0).unwrap();
        assert!((first.x as f64 - 4.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn boundary_reflects_velocity_with_restitution() {
        let mut c = ParticleConfig {
            speed: 10.0,
            frequency: 10.0,
            direction_degrees: 90.0,
            spread_degrees: 0.0,
            ..Default::default()
        };
        c.p3.boundary.enabled = true;
        c.p3.boundary.min = [-5.0, -100.0, -100.0];
        c.p3.boundary.max = [5.0, 100.0, 100.0];
        c.p3.boundary.restitution = 1.0;
        let first = sample(&c, 1.0).into_iter().find(|p| p.id == 0).unwrap();
        assert!(first.x.abs() < 0.01);
    }

    #[test]
    fn dispersion_and_stop_use_exact_event_times() {
        let mut c = ParticleConfig {
            speed: 0.0,
            frequency: 10.0,
            ..Default::default()
        };
        c.p3.dispersion.enabled = true;
        c.p3.dispersion.after = 0.5;
        c.p3.dispersion.impulse = 10.0;
        c.p3.dispersion.stop_after = 1.0;
        let first = sample(&c, 1.5).into_iter().find(|p| p.id == 0).unwrap();
        assert!((first.y + 5.0).abs() < 0.01);
    }

    #[test]
    fn convergence_can_stop_or_remove_particles() {
        let mut c = ParticleConfig {
            speed: 0.0,
            frequency: 10.0,
            ..Default::default()
        };
        c.p3.convergence.enabled = true;
        c.p3.convergence.target = [10.0, 0.0, 0.0];
        c.p3.convergence.strength = 100.0;
        c.p3.convergence.radius = 1.0;
        c.p3.convergence.arrival = Arrival::Stop;
        let at_one = sample(&c, 1.0).into_iter().find(|p| p.id == 0).unwrap();
        let at_two = sample(&c, 2.0).into_iter().find(|p| p.id == 0).unwrap();
        assert_eq!(at_one.x, at_two.x);
        c.p3.convergence.arrival = Arrival::Vanish;
        assert!(sample(&c, 1.0).into_iter().all(|p| p.id != 0));
    }

    #[test]
    fn orbit_and_frequency_modulation_are_deterministic() {
        let mut c = ParticleConfig {
            speed: 0.0,
            frequency: 100.0,
            ..Default::default()
        };
        c.p3.orbit.plane = OrbitPlane::Xy;
        c.p3.orbit.radius = 10.0;
        c.p3.orbit.angular_speed_degrees = 90.0;
        c.p3.modulation.frequency = Wave {
            depth: 0.9,
            period: 2.0,
        };
        assert!(
            sample(&c, 0.5).len()
                > sample(
                    &ParticleConfig {
                        p3: P3Config::default(),
                        ..c
                    },
                    0.5
                )
                .len()
        );
        let first = sample(&c, 1.0).into_iter().find(|p| p.id == 0).unwrap();
        assert!(first.x.abs() < 0.001);
        assert!((first.y - 10.0).abs() < 0.001);
        assert_eq!(sample(&c, 2.0).len(), 21);
    }

    #[test]
    fn time_warp_changes_motion_without_changing_lifetime() {
        let mut c = ParticleConfig {
            speed: 10.0,
            direction_degrees: 90.0,
            spread_degrees: 0.0,
            frequency: 10.0,
            lifetime: 1.0,
            ..Default::default()
        };
        c.p3.time_warp.scale = 2.0;
        c.p3.rotation_acceleration[0] = 100.0;
        let first = sample(&c, 0.5).into_iter().find(|p| p.id == 0).unwrap();
        assert!((first.x - 10.0).abs() < 0.001);
        assert!((first.rx - 50.0).abs() < 0.001);
        assert!(sample(&c, 1.0).into_iter().all(|p| p.id != 0));
    }

    #[test]
    fn appearance_waves_use_motion_age() {
        let mut c = ParticleConfig {
            frequency: 10.0,
            alpha_start: 0.4,
            alpha_end: 0.4,
            zoom_start: 1.0,
            zoom_end: 1.0,
            ..Default::default()
        };
        c.p3.modulation.alpha = Wave {
            depth: 0.5,
            period: 1.0,
        };
        c.p3.modulation.zoom = Wave {
            depth: 0.5,
            period: 1.0,
        };
        let first = sample(&c, 0.25).into_iter().find(|p| p.id == 0).unwrap();
        assert!((first.alpha - 0.6).abs() < 0.001);
        assert!((first.scale - 1.5).abs() < 0.001);
    }

    #[test]
    fn long_wind_integration_reaches_requested_time() {
        let mut c = ParticleConfig {
            speed: 0.0,
            lifetime: 100.0,
            ..Default::default()
        };
        c.p3.wind.from = [2.0, 0.0, 0.0];
        let pos =
            c.p3.position(0.0, 60.0, [0.0; 3], [0.0; 3], [0.0; 3], [0.0; 3])
                .unwrap();
        assert!((pos[0] - 3600.0).abs() < 0.001);
    }

    #[test]
    fn trails_and_combined_features_survive_out_of_order_seeks() {
        let mut c = ParticleConfig {
            frequency: 10.0,
            ..Default::default()
        };
        c.p3.trail.mode = TrailMode::Afterimage;
        c.p3.trail.length = 0.5;
        c.p3.trail.samples = 4;
        c.p3.variation.speed_percent = 20.0;
        c.p3.wind.from = [2.0, 0.0, 0.0];
        c.p3.boundary.enabled = true;
        c.p3.dispersion.enabled = true;
        c.p3.dispersion.after = 0.4;
        c.p3.dispersion.impulse = 5.0;
        c.p3.orbit.plane = OrbitPlane::Xz;
        c.p3.orbit.radius = 5.0;
        let first = render_batch(&c, 1.25);
        assert!(!first.particles.is_empty());
        assert!(!first.trail_images.is_empty());
        let _ = render_batch(&c, 2.75);
        assert_eq!(first, render_batch(&c, 1.25));
        assert!(!render_batch(&c, 3.1).trail_images.is_empty());
    }
}
