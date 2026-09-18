//! Deterministic P3 motion features. Their ordering is a port convention until
//! a trace from the original host can establish the legacy event order.

use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, Debug)]
pub struct WindCurve {
    pub from: [f64; 3],
    pub to: [f64; 3],
    /// The endpoint is reached after this many seconds of object time.
    pub duration: f64,
    /// Damping coefficient in 1/s.
    pub drag: f64,
}

impl Default for WindCurve {
    fn default() -> Self {
        Self {
            from: [0.0; 3],
            to: [0.0; 3],
            duration: 0.0,
            drag: 0.0,
        }
    }
}

impl WindCurve {
    pub fn at(&self, object_time: f64) -> [f64; 3] {
        let progress = if self.duration > 0.0 {
            (object_time / self.duration).clamp(0.0, 1.0)
        } else {
            0.0
        };
        std::array::from_fn(|axis| self.from[axis] + (self.to[axis] - self.from[axis]) * progress)
    }

    fn active(&self) -> bool {
        self.from != [0.0; 3] || self.to != [0.0; 3] || self.drag != 0.0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Variation {
    pub speed_percent: f64,
    pub lifetime_percent: f64,
    pub rotation_percent: f64,
    pub gravity_percent: f64,
    pub alpha_percent: f64,
    pub zoom_percent: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Wave {
    /// Symmetric amplitude as a fraction of the underlying value.
    pub depth: f64,
    pub period: f64,
}

impl Default for Wave {
    fn default() -> Self {
        Self {
            depth: 0.0,
            period: 1.0,
        }
    }
}

impl Wave {
    pub fn value(&self, time: f64) -> f64 {
        if self.depth == 0.0 || self.period <= 0.0 {
            0.0
        } else {
            self.depth * (TAU * time / self.period).sin()
        }
    }

    fn derivative(&self, time: f64) -> f64 {
        if self.depth == 0.0 || self.period <= 0.0 {
            0.0
        } else {
            let omega = TAU / self.period;
            self.depth * omega * (omega * time).cos()
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Modulation {
    pub frequency: Wave,
    pub speed: Wave,
    pub alpha: Wave,
    pub zoom: Wave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrival {
    Continue,
    Stop,
    Vanish,
}

#[derive(Clone, Copy, Debug)]
pub struct Convergence {
    pub enabled: bool,
    pub target: [f64; 3],
    pub strength: f64,
    pub start: f64,
    pub radius: f64,
    pub arrival: Arrival,
}

impl Default for Convergence {
    fn default() -> Self {
        Self {
            enabled: false,
            target: [0.0; 3],
            strength: 0.0,
            start: 0.0,
            radius: 0.0,
            arrival: Arrival::Continue,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Boundary {
    pub enabled: bool,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub restitution: f64,
}

impl Default for Boundary {
    fn default() -> Self {
        Self {
            enabled: false,
            min: [-640.0, -360.0, -500.0],
            max: [640.0, 360.0, 500.0],
            restitution: 0.9,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Dispersion {
    pub enabled: bool,
    pub after: f64,
    pub impulse: f64,
    pub xy_spread_degrees: f64,
    pub z_spread_degrees: f64,
    pub on_bounce: bool,
    /// Zero disables sudden stopping.
    pub stop_after: f64,
}

impl Default for Dispersion {
    fn default() -> Self {
        Self {
            enabled: false,
            after: 1.0,
            impulse: 0.0,
            xy_spread_degrees: 0.0,
            z_spread_degrees: 0.0,
            on_bounce: false,
            stop_after: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbitPlane {
    Off,
    Xy,
    Xz,
    Yz,
    Sphere,
}

#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub plane: OrbitPlane,
    pub center: [f64; 3],
    pub radius: f64,
    pub radial_speed: f64,
    pub angular_speed_degrees: f64,
    pub elevation_speed_degrees: f64,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            plane: OrbitPlane::Off,
            center: [0.0; 3],
            radius: 0.0,
            radial_speed: 0.0,
            angular_speed_degrees: 60.0,
            elevation_speed_degrees: 0.0,
        }
    }
}

impl Orbit {
    fn apply(&self, pos: [f64; 3], time: f64) -> [f64; 3] {
        if self.plane == OrbitPlane::Off {
            return pos;
        }
        let mut delta: [f64; 3] = std::array::from_fn(|axis| pos[axis] - self.center[axis]);
        let radius = self.radius + self.radial_speed * time;
        let angle = self.angular_speed_degrees.to_radians() * time;
        let elevation = self.elevation_speed_degrees.to_radians() * time;
        let (s, c) = angle.sin_cos();
        match self.plane {
            OrbitPlane::Off => unreachable!(),
            OrbitPlane::Xy | OrbitPlane::Sphere => {
                delta[0] += radius;
                (delta[0], delta[1]) = (delta[0] * c - delta[1] * s, delta[0] * s + delta[1] * c);
                if self.plane == OrbitPlane::Sphere {
                    let (se, ce) = elevation.sin_cos();
                    (delta[0], delta[2]) =
                        (delta[0] * ce - delta[2] * se, delta[0] * se + delta[2] * ce);
                }
            }
            OrbitPlane::Xz => {
                delta[0] += radius;
                (delta[0], delta[2]) = (delta[0] * c - delta[2] * s, delta[0] * s + delta[2] * c);
            }
            OrbitPlane::Yz => {
                delta[1] += radius;
                (delta[1], delta[2]) = (delta[1] * c - delta[2] * s, delta[1] * s + delta[2] * c);
            }
        }
        std::array::from_fn(|axis| delta[axis] + self.center[axis])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TimeWarp {
    pub scale: f64,
    pub offset: f64,
}

impl Default for TimeWarp {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: 0.0,
        }
    }
}

impl TimeWarp {
    pub fn motion_age(&self, real_age: f64) -> f64 {
        ((real_age + self.offset) * self.scale).max(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailMode {
    Off,
    Afterimage,
    Ribbon,
    Points,
}

#[derive(Clone, Copy, Debug)]
pub struct Trail {
    pub mode: TrailMode,
    pub length: f64,
    pub samples: u32,
    pub opacity: f64,
    pub scale: f64,
    pub width: f64,
}

impl Default for Trail {
    fn default() -> Self {
        Self {
            mode: TrailMode::Off,
            length: 0.1,
            samples: 2,
            opacity: 0.3,
            scale: 0.3,
            width: 10.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct P3Config {
    pub wind: WindCurve,
    pub variation: Variation,
    pub modulation: Modulation,
    pub convergence: Convergence,
    pub boundary: Boundary,
    pub dispersion: Dispersion,
    pub orbit: Orbit,
    pub time_warp: TimeWarp,
    pub trail: Trail,
    pub rotation_acceleration: [f64; 3],
}

impl P3Config {
    pub(crate) fn needs_integration(&self) -> bool {
        self.wind.active()
            || self.convergence.enabled
            || self.boundary.enabled
            || self.dispersion.enabled
            || self.dispersion.stop_after > 0.0
            || self.modulation.speed.depth != 0.0
    }

    pub(crate) fn position(
        &self,
        birth_time: f64,
        real_age: f64,
        origin: [f64; 3],
        initial_velocity: [f64; 3],
        gravity: [f64; 3],
        dispersion_impulse: [f64; 3],
    ) -> Option<[f64; 3]> {
        let age = self.time_warp.motion_age(real_age);
        if !age.is_finite() {
            return None;
        }
        let pos = if self.needs_integration() {
            self.integrate(
                birth_time,
                age,
                origin,
                initial_velocity,
                gravity,
                dispersion_impulse,
            )?
        } else {
            std::array::from_fn(|axis| {
                origin[axis] + initial_velocity[axis] * age + 0.5 * gravity[axis] * age * age
            })
        };
        Some(self.orbit.apply(pos, age))
    }

    fn integrate(
        &self,
        birth_time: f64,
        age: f64,
        origin: [f64; 3],
        initial_velocity: [f64; 3],
        gravity: [f64; 3],
        impulse: [f64; 3],
    ) -> Option<[f64; 3]> {
        let mut pos = origin;
        let mut vel = initial_velocity;
        let mut t = 0.0;
        let mut dispersed = false;
        // At most 4096 regular steps plus event threshold splits and rounding.
        // Fixed time origins keep repeated seeks deterministic.
        let step = (1.0_f64 / 120.0).max(age / 4096.0);
        for _ in 0..4112 {
            if self.dispersion.enabled
                && !self.dispersion.on_bounce
                && !dispersed
                && t >= self.dispersion.after
            {
                for axis in 0..3 {
                    vel[axis] += impulse[axis];
                }
                dispersed = true;
            }
            if self.dispersion.stop_after > 0.0 && t >= self.dispersion.stop_after {
                break;
            }
            if self.arrived(pos, t) {
                match self.convergence.arrival {
                    Arrival::Continue => {}
                    Arrival::Stop => break,
                    Arrival::Vanish => return None,
                }
            }
            if t >= age {
                break;
            }
            let mut next = (t + step).min(age);
            if self.dispersion.enabled && !self.dispersion.on_bounce && !dispersed {
                split_at(&mut next, t, self.dispersion.after);
            }
            if self.dispersion.stop_after > 0.0 {
                split_at(&mut next, t, self.dispersion.stop_after);
            }
            if self.convergence.enabled {
                split_at(&mut next, t, self.convergence.start);
            }
            let dt = next - t;
            if dt <= 0.0 || !dt.is_finite() {
                return None;
            }
            let mid = (t + next) * 0.5;
            let wind = self.wind.at(birth_time + mid);
            let mut acc: [f64; 3] =
                std::array::from_fn(|axis| gravity[axis] + wind[axis] - self.wind.drag * vel[axis]);
            if self.convergence.enabled && mid >= self.convergence.start {
                let delta: [f64; 3] =
                    std::array::from_fn(|axis| self.convergence.target[axis] - pos[axis]);
                let distance = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
                if distance > 1e-9 {
                    for axis in 0..3 {
                        acc[axis] += delta[axis] / distance * self.convergence.strength;
                    }
                }
            }
            if self.modulation.speed.depth != 0.0 {
                let speed_change = self.modulation.speed.derivative(mid);
                for axis in 0..3 {
                    acc[axis] += initial_velocity[axis] * speed_change;
                }
            }
            for axis in 0..3 {
                pos[axis] += vel[axis] * dt + 0.5 * acc[axis] * dt * dt;
                vel[axis] += acc[axis] * dt;
            }
            let bounced = self.reflect(&mut pos, &mut vel);
            if bounced && self.dispersion.enabled && self.dispersion.on_bounce && !dispersed {
                for axis in 0..3 {
                    vel[axis] += impulse[axis];
                }
                dispersed = true;
            }
            t = next;
        }
        if t < age
            && !(self.dispersion.stop_after > 0.0 && t >= self.dispersion.stop_after)
            && !(self.convergence.arrival == Arrival::Stop && self.arrived(pos, t))
        {
            return None;
        }
        if self.arrived(pos, age) && self.convergence.arrival == Arrival::Vanish {
            return None;
        }
        Some(pos)
    }

    fn arrived(&self, pos: [f64; 3], age: f64) -> bool {
        self.convergence.enabled
            && age >= self.convergence.start
            && self.convergence.radius > 0.0
            && pos
                .iter()
                .enumerate()
                .map(|(axis, v)| (v - self.convergence.target[axis]).powi(2))
                .sum::<f64>()
                <= self.convergence.radius.powi(2)
    }

    fn reflect(&self, pos: &mut [f64; 3], vel: &mut [f64; 3]) -> bool {
        if !self.boundary.enabled {
            return false;
        }
        let mut bounced = false;
        for axis in 0..3 {
            let min = self.boundary.min[axis];
            let max = self.boundary.max[axis];
            if min >= max {
                continue;
            }
            for _ in 0..8 {
                if pos[axis] < min {
                    pos[axis] = 2.0 * min - pos[axis];
                    vel[axis] = vel[axis].abs() * self.boundary.restitution;
                    bounced = true;
                } else if pos[axis] > max {
                    pos[axis] = 2.0 * max - pos[axis];
                    vel[axis] = -vel[axis].abs() * self.boundary.restitution;
                    bounced = true;
                } else {
                    break;
                }
            }
            pos[axis] = pos[axis].clamp(min, max);
        }
        bounced
    }
}

fn split_at(next: &mut f64, current: f64, event: f64) {
    if event > current && event < *next {
        *next = event;
    }
}

pub(crate) fn dispersion_vector(
    impulse: f64,
    xy_spread_degrees: f64,
    z_spread_degrees: f64,
    random_xy: f64,
    random_z: f64,
) -> [f64; 3] {
    let xy = (random_xy * 2.0 - 1.0) * xy_spread_degrees * PI / 180.0;
    let z = (random_z * 2.0 - 1.0) * z_spread_degrees * PI / 180.0;
    [
        xy.sin() * z.cos() * impulse,
        -xy.cos() * z.cos() * impulse,
        z.sin() * impulse,
    ]
}
