//! P4 host/link safety primitives.
//!
//! These types deliberately do not call a host API. They make reference
//! validation and time selection deterministic before a host-specific adapter
//! resolves an AviUtl2 layer or material.

use crate::ParticleSample;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Source,
    Mask,
    SharedWind,
    Time2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkRequest {
    pub kind: LinkKind,
    /// `None` means the link is disabled. Layer numbers are host layer IDs.
    pub target_layer: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkError {
    SelfReference,
    MissingTarget,
    Cycle,
}

/// A bounded link graph used by host adapters before dereferencing objects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkGraph {
    edges: Vec<(u32, u32)>,
}

impl LinkGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one link only when both endpoints are known and no cycle is made.
    pub fn try_add(&mut self, from: u32, to: u32, known_layers: &[u32]) -> Result<(), LinkError> {
        if from == to {
            return Err(LinkError::SelfReference);
        }
        if !known_layers.contains(&to) {
            return Err(LinkError::MissingTarget);
        }
        if self.would_cycle(from, to) {
            return Err(LinkError::Cycle);
        }
        self.edges.push((from, to));
        Ok(())
    }

    pub fn target(&self, from: u32) -> Option<u32> {
        self.edges
            .iter()
            .find(|(source, _)| *source == from)
            .map(|(_, target)| *target)
    }

    fn would_cycle(&self, from: u32, to: u32) -> bool {
        let mut current = to;
        for _ in 0..self.edges.len().saturating_add(1) {
            if current == from {
                return true;
            }
            let Some(next) = self.target(current) else {
                return false;
            };
            current = next;
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P4TimeContext {
    pub scene_time: f64,
    pub object_time: f64,
    pub birth_time: f64,
    pub particle_age: f64,
}

impl P4TimeContext {
    pub fn new(scene_time: f64, object_time: f64, birth_time: f64) -> Self {
        Self {
            scene_time,
            object_time,
            birth_time,
            particle_age: object_time - birth_time,
        }
    }

    /// Select the documented time domain for a curve or material lookup.
    pub fn at(&self, domain: TimeDomain) -> f64 {
        match domain {
            TimeDomain::Scene => self.scene_time,
            TimeDomain::Object => self.object_time,
            TimeDomain::Birth => self.birth_time,
            TimeDomain::ParticleAge => self.particle_age,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeDomain {
    Scene,
    Object,
    Birth,
    ParticleAge,
}

/// An RGBA alpha plane sampled from a host image at the current object time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlphaMask {
    width: u32,
    height: u32,
    alpha: Vec<u8>,
}

impl AlphaMask {
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Option<Self> {
        let pixels = (width as usize).checked_mul(height as usize)?;
        if rgba.len() != pixels.checked_mul(4)? {
            return None;
        }
        Some(Self {
            width,
            height,
            alpha: rgba.chunks_exact(4).map(|pixel| pixel[3]).collect(),
        })
    }

    /// Test a particle coordinate against a mask centred on the scene origin.
    /// Coordinates outside the image are always transparent.
    pub fn contains_centered(&self, x: f32, y: f32, threshold: u8) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let px = (x + self.width as f32 * 0.5).floor() as i64;
        let py = (y + self.height as f32 * 0.5).floor() as i64;
        if px < 0 || py < 0 || px >= self.width as i64 || py >= self.height as i64 {
            return false;
        }
        self.alpha[py as usize * self.width as usize + px as usize] >= threshold
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskClip {
    Disabled,
    HideInside,
    HideOutside,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskCollisionMode {
    Bounce,
    Stop,
    Vanish,
}

#[derive(Clone, Copy, Debug)]
pub struct MaskCollision<'a> {
    pub mask: &'a AlphaMask,
    pub threshold: u8,
    pub mode: MaskCollisionMode,
    pub restitution: f64,
}

impl MaskCollision<'_> {
    pub(crate) fn first_hit(&self, previous: [f64; 3], next: [f64; 3]) -> Option<[f64; 3]> {
        if self
            .mask
            .contains_centered(previous[0] as f32, previous[1] as f32, self.threshold)
        {
            return None;
        }
        let distance = (next[0] - previous[0]).hypot(next[1] - previous[1]);
        let steps = (distance.ceil() as usize).clamp(1, 128);
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let point =
                std::array::from_fn(|axis| previous[axis] + (next[axis] - previous[axis]) * t);
            if self
                .mask
                .contains_centered(point[0] as f32, point[1] as f32, self.threshold)
            {
                return Some(point);
            }
        }
        None
    }

    pub(crate) fn reflect(&self, previous: [f64; 3], next: [f64; 3], velocity: &mut [f64; 3]) {
        let x_cross =
            self.mask
                .contains_centered(next[0] as f32, previous[1] as f32, self.threshold);
        let y_cross =
            self.mask
                .contains_centered(previous[0] as f32, next[1] as f32, self.threshold);
        if x_cross || !y_cross {
            velocity[0] *= -self.restitution.clamp(0.0, 2.0);
        }
        if y_cross || !x_cross {
            velocity[1] *= -self.restitution.clamp(0.0, 2.0);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshSegment {
    pub from: [f32; 3],
    pub to: [f32; 3],
}

/// A bounded face fan using the first two successors within range.
pub fn build_mesh_faces(
    particles: &[ParticleSample],
    max_distance: f32,
    max_particles: usize,
) -> Vec<[[f32; 3]; 3]> {
    if !max_distance.is_finite() || max_distance <= 0.0 {
        return Vec::new();
    }
    let points = &particles[..particles.len().min(max_particles)];
    let limit = max_distance * max_distance;
    let mut faces = Vec::new();
    for (index, parent) in points.iter().enumerate() {
        let mut near = Vec::with_capacity(2);
        for child in &points[index + 1..] {
            let dx = child.x - parent.x;
            let dy = child.y - parent.y;
            let dz = child.z - parent.z;
            if dx * dx + dy * dy + dz * dz <= limit {
                near.push(child);
            }
            if near.len() == 2 {
                break;
            }
        }
        if near.len() == 2 {
            faces.push([
                [parent.x, parent.y, parent.z],
                [near[0].x, near[0].y, near[0].z],
                [near[1].x, near[1].y, near[1].z],
            ]);
        }
    }
    faces
}

/// Build a deterministic, bounded particle mesh.
///
/// Candidates retain their emission order. This is intentionally bounded to
/// avoid an unbounded O(n²) cost when the emitter has 10,000 live particles.
pub fn build_mesh(
    particles: &[ParticleSample],
    max_distance: f32,
    max_links_per_particle: usize,
    max_particles: usize,
) -> Vec<MeshSegment> {
    if !max_distance.is_finite() || max_distance <= 0.0 || max_links_per_particle == 0 {
        return Vec::new();
    }
    let particles = &particles[..particles.len().min(max_particles)];
    let max_distance_squared = max_distance * max_distance;
    let mut segments = Vec::new();
    for (index, from) in particles.iter().enumerate() {
        let mut links = 0;
        for to in &particles[index + 1..] {
            let dx = to.x - from.x;
            let dy = to.y - from.y;
            let dz = to.z - from.z;
            if dx * dx + dy * dy + dz * dz <= max_distance_squared {
                segments.push(MeshSegment {
                    from: [from.x, from.y, from.z],
                    to: [to.x, to.y, to.z],
                });
                links += 1;
                if links == max_links_per_particle {
                    break;
                }
            }
        }
    }
    segments
}

/// Create bounded ring children around each sampled parent particle.
///
/// This is a P4 interim funnel model: the children use the same source image
/// and inherit the parent age, alpha, and rotation. It does not claim the old
/// DLL's multi-stage funnel timing or circle settings.
pub fn build_funnel(
    parents: &[ParticleSample],
    object_time: f64,
    count: usize,
    radius: f32,
    scale: f32,
    angular_speed_degrees: f32,
    max_particles: usize,
) -> Vec<ParticleSample> {
    if !object_time.is_finite()
        || count == 0
        || !radius.is_finite()
        || !scale.is_finite()
        || !angular_speed_degrees.is_finite()
        || max_particles == 0
    {
        return Vec::new();
    }
    let count = count.min(128);
    let parent_limit = parents
        .len()
        .min(max_particles.saturating_add(count - 1) / count);
    let mut children = Vec::with_capacity(max_particles.min(parent_limit * count));
    let spin = angular_speed_degrees.to_radians() * object_time as f32;
    for parent in &parents[..parent_limit] {
        let seed_phase = (parent.id as f64 * 0.618_033_988_749_894_9) as f32;
        for child_index in 0..count {
            if children.len() == max_particles {
                return children;
            }
            let phase =
                std::f32::consts::TAU * child_index as f32 / count as f32 + spin + seed_phase;
            let mut child = *parent;
            child.id = parent
                .id
                .saturating_mul(128)
                .saturating_add(child_index as u64);
            child.x += phase.cos() * radius;
            child.y += phase.sin() * radius;
            child.scale = (child.scale * scale).max(0.0);
            children.push(child);
        }
    }
    children
}

impl MaskClip {
    pub fn keeps(self, mask: &AlphaMask, x: f32, y: f32, threshold: u8) -> bool {
        let inside = mask.contains_centered(x, y, threshold);
        match self {
            Self::Disabled => true,
            Self::HideInside => !inside,
            Self::HideOutside => inside,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_self_links_are_rejected() {
        let mut graph = LinkGraph::new();
        assert_eq!(graph.try_add(1, 99, &[1, 2]), Err(LinkError::MissingTarget));
        assert_eq!(graph.try_add(1, 1, &[1, 2]), Err(LinkError::SelfReference));
    }

    #[test]
    fn cycles_are_rejected_without_mutating_graph() {
        let mut graph = LinkGraph::new();
        graph.try_add(1, 2, &[1, 2, 3]).unwrap();
        graph.try_add(2, 3, &[1, 2, 3]).unwrap();
        assert_eq!(graph.try_add(3, 1, &[1, 2, 3]), Err(LinkError::Cycle));
        assert_eq!(graph.target(3), None);
    }

    #[test]
    fn time_domains_are_explicit_and_seek_stable() {
        let context = P4TimeContext::new(42.0, 3.5, 2.0);
        assert_eq!(context.at(TimeDomain::Scene), 42.0);
        assert_eq!(context.at(TimeDomain::Object), 3.5);
        assert_eq!(context.at(TimeDomain::Birth), 2.0);
        assert_eq!(context.at(TimeDomain::ParticleAge), 1.5);
    }

    #[test]
    fn alpha_mask_uses_scene_centred_coordinates() {
        let rgba = [
            0, 0, 0, 0, 0, 0, 0, 0, // top row
            0, 0, 0, 0, 0, 0, 0, 255, // bottom-right is opaque
        ];
        let mask = AlphaMask::from_rgba(2, 2, &rgba).unwrap();
        assert!(mask.contains_centered(0.0, 0.0, 1));
        assert!(!mask.contains_centered(-1.0, -1.0, 1));
        assert!(!mask.contains_centered(9.0, 9.0, 1));
        assert!(MaskClip::HideOutside.keeps(&mask, 0.0, 0.0, 1));
        assert!(!MaskClip::HideInside.keeps(&mask, 0.0, 0.0, 1));
    }

    #[test]
    fn mask_detects_a_thin_boundary_between_transparent_endpoints() {
        let mut rgba = vec![0_u8; 5 * 4];
        rgba[2 * 4 + 3] = 255;
        let mask = AlphaMask::from_rgba(5, 1, &rgba).unwrap();
        let collision = MaskCollision {
            mask: &mask,
            threshold: 1,
            mode: MaskCollisionMode::Bounce,
            restitution: 1.0,
        };
        assert!(
            collision
                .first_hit([-2.0, 0.0, 0.0], [2.0, 0.0, 0.0])
                .is_some()
        );
    }

    #[test]
    fn mask_collision_replays_bounce_stop_and_vanish_on_seek() {
        let mut rgba = vec![0_u8; 4 * 4 * 4];
        for row in 0..4 {
            rgba[(row * 4 + 3) * 4 + 3] = 255;
        }
        let mask = AlphaMask::from_rgba(4, 4, &rgba).unwrap();
        let config = crate::ParticleConfig {
            speed: 2.0,
            direction_degrees: 90.0,
            spread_degrees: 0.0,
            frequency: 10.0,
            ..Default::default()
        };
        let collision = MaskCollision {
            mask: &mask,
            threshold: 1,
            mode: MaskCollisionMode::Bounce,
            restitution: 1.0,
        };
        let first = crate::render_batch_with_mask(&config, 1.0, collision);
        let bounced = first.particles.iter().find(|p| p.id == 0).unwrap();
        assert!(bounced.x.abs() < 0.1);
        let _ = crate::render_batch_with_mask(&config, 2.0, collision);
        assert_eq!(
            first,
            crate::render_batch_with_mask(&config, 1.0, collision)
        );

        let stopped = crate::render_batch_with_mask(
            &config,
            1.0,
            MaskCollision {
                mode: MaskCollisionMode::Stop,
                ..collision
            },
        );
        let stopped = stopped.particles.iter().find(|p| p.id == 0).unwrap();
        assert!(stopped.x > 0.0 && stopped.x < 1.0);
        let vanished = crate::render_batch_with_mask(
            &config,
            1.0,
            MaskCollision {
                mode: MaskCollisionMode::Vanish,
                ..collision
            },
        );
        assert!(vanished.particles.iter().all(|p| p.id != 0));
    }

    #[test]
    fn mesh_is_bounded_and_uses_particle_order() {
        let particles = [
            ParticleSample {
                id: 1,
                birth_time: 0.0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                rx: 0.0,
                ry: 0.0,
                rz: 0.0,
                scale: 1.0,
                alpha: 1.0,
            },
            ParticleSample {
                id: 2,
                birth_time: 0.0,
                x: 5.0,
                y: 0.0,
                z: 0.0,
                rx: 0.0,
                ry: 0.0,
                rz: 0.0,
                scale: 1.0,
                alpha: 1.0,
            },
            ParticleSample {
                id: 3,
                birth_time: 0.0,
                x: 8.0,
                y: 0.0,
                z: 0.0,
                rx: 0.0,
                ry: 0.0,
                rz: 0.0,
                scale: 1.0,
                alpha: 1.0,
            },
        ];
        let mesh = build_mesh(&particles, 10.0, 1, 2);
        assert_eq!(mesh.len(), 1);
        assert_eq!(mesh[0].from, [0.0, 0.0, 0.0]);
        assert_eq!(mesh[0].to, [5.0, 0.0, 0.0]);
    }

    #[test]
    fn funnel_children_are_deterministic_and_bounded() {
        let parent = ParticleSample {
            id: 7,
            birth_time: 0.0,
            x: 10.0,
            y: 20.0,
            z: 0.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            scale: 2.0,
            alpha: 1.0,
        };
        let children = build_funnel(&[parent], 0.0, 4, 10.0, 0.5, 0.0, 3);
        assert_eq!(children.len(), 3);
        assert!(children.iter().all(|child| child.scale == 1.0));
        assert_eq!(children, build_funnel(&[parent], 0.0, 4, 10.0, 0.5, 0.0, 3));
    }
}
