//! P4 material selection and host drawing helpers.

use aviutl2::{
    AnyResult,
    filter::{
        DrawImageParam, FilterProcVideo, ImageResource, VertexColor, VertexList, VertexTexture,
    },
};
use particle_core::ParticleSample;
use std::{collections::HashMap, path::PathBuf};

use crate::FilterConfig;

pub(super) fn sequence_path(pattern: &str, frame: u64) -> Option<PathBuf> {
    let bytes = pattern.as_bytes();
    let first = bytes.iter().position(|byte| *byte == b'#')?;
    let last = first
        + bytes[first..]
            .iter()
            .take_while(|byte| **byte == b'#')
            .count();
    if last - first > 12 || pattern.len() > 4096 {
        return None;
    }
    let number = format!("{:0width$}", frame, width = last - first);
    Some(PathBuf::from(format!(
        "{}{}{}",
        &pattern[..first],
        number,
        &pattern[last..]
    )))
}

pub(super) struct DrawContext<'a> {
    pub source: ImageResource,
    pub config: &'a FilterConfig,
    pub object_time: f64,
    pub background: Option<ImageResource>,
    pub sequence: HashMap<u64, Option<ImageResource>>,
}

impl DrawContext<'_> {
    pub fn draw_many(
        &mut self,
        video: &mut FilterProcVideo<()>,
        particles: Vec<ParticleSample>,
    ) -> AnyResult<()> {
        const QUADS_PER_CALL: usize = 2_048;
        let can_batch = self.config.source_kind == 0
            && self.config.particle_effect_name.trim().is_empty()
            && particles
                .iter()
                .all(|particle| particle.rx.abs() <= 1e-6 && particle.ry.abs() <= 1e-6);
        if can_batch
            && let Ok((width, height)) = video.get_image_resource_size(&self.source)
            && width > 0
            && height > 0
        {
            for particles in particles.chunks(QUADS_PER_CALL) {
                let quads = particles
                    .iter()
                    .copied()
                    .map(|particle| {
                        quad(
                            particle,
                            width as f32 * particle.scale,
                            height as f32 * particle.scale,
                            [0.0, 0.0, 1.0, 1.0],
                        )
                    })
                    .collect();
                video.draw_poly(&VertexList::QuadTexture(quads), Some(&self.source))?;
            }
            return Ok(());
        }
        for particle in particles {
            self.draw(video, particle)?;
        }
        Ok(())
    }

    pub fn draw(
        &mut self,
        video: &mut FilterProcVideo<()>,
        particle: ParticleSample,
    ) -> AnyResult<()> {
        match self.config.source_kind {
            1 => self.draw_sequence(video, particle),
            2 | 3 => self.draw_text_cell(video, particle),
            4 => self.draw_solid(video, particle),
            5 => self.draw_glass(video, particle),
            6 => self.draw_image_set(video, particle),
            _ => self.draw_image(video, &self.source.clone(), particle),
        }
    }

    fn draw_image_set(
        &mut self,
        video: &mut FilterProcVideo<()>,
        particle: ParticleSample,
    ) -> AnyResult<()> {
        let files: Vec<&str> = self
            .config
            .image_files
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .take(4096)
            .collect();
        if files.is_empty() {
            return self.draw_image(video, &self.source.clone(), particle);
        }
        let selection = if self.config.image_random != 0 {
            // Stable integer mixing keeps the selection deterministic while
            // reproducing the legacy 「ぱらばら」 intent.
            let mut value = particle.id ^ 0x9e37_79b9_7f4a_7c15;
            value ^= value >> 30;
            value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value ^= value >> 27;
            value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
            (value ^ (value >> 31)) as usize % files.len()
        } else {
            particle.id as usize % files.len()
        };
        let key = selection as u64;
        if !self.sequence.contains_key(&key) {
            let path = PathBuf::from(files[selection]);
            let source = path
                .is_file()
                .then_some(ImageResource::ImageFile(path.clone()));
            if source.is_none() {
                crate::p4_host::warn_once(
                    format!("image-set-{}", path.display()),
                    format!(
                        "パーティクル(R): 画像素材が見つかりません: {}",
                        path.display()
                    ),
                );
            }
            self.sequence.insert(key, source);
        }
        if let Some(Some(source)) = self.sequence.get(&key) {
            self.draw_image(video, source, particle)
        } else {
            self.draw_image(video, &self.source.clone(), particle)
        }
    }

    fn draw_sequence(
        &mut self,
        video: &mut FilterProcVideo<()>,
        particle: ParticleSample,
    ) -> AnyResult<()> {
        let time = match self.config.source_time {
            0 => self.object_time,
            1 => particle.birth_time,
            _ => self.object_time - particle.birth_time,
        };
        if !time.is_finite() || time < 0.0 {
            return Ok(());
        }
        let frame = (time * self.config.sequence_fps.clamp(1, 240) as f64).floor() as u64;
        if !self.sequence.contains_key(&frame) && self.sequence.len() < 4096 {
            let source = sequence_path(&self.config.sequence_pattern, frame)
                .or_else(|| {
                    let path = PathBuf::from(&self.config.sequence_pattern);
                    (!self.config.sequence_pattern.contains('#')).then_some(path)
                })
                .filter(|path| path.is_file())
                .map(ImageResource::ImageFile);
            if source.is_none() {
                crate::p4_host::warn_once(
                    format!("sequence-{}", self.config.sequence_pattern),
                    format!(
                        "パーティクル(R): 連番素材が見つかりません: {}",
                        self.config.sequence_pattern
                    ),
                );
            }
            self.sequence.insert(frame, source);
        }
        if let Some(Some(source)) = self.sequence.get(&frame) {
            self.draw_image(video, source, particle)
        } else {
            self.draw_image(video, &self.source.clone(), particle)
        }
    }

    fn draw_image(
        &self,
        video: &mut FilterProcVideo<()>,
        source: &ImageResource,
        particle: ParticleSample,
    ) -> AnyResult<()> {
        if !self.config.particle_effect_name.trim().is_empty() {
            let effect_source =
                ImageResource::Resource(format!("particle2r-effect-{}", video.object.id));
            if video.copy_image_resource(source, &effect_source).is_ok() {
                let params = effect_params(
                    &self.config.particle_effect_params,
                    particle,
                    self.object_time,
                );
                if video
                    .exec_effect(
                        self.config.particle_effect_name.trim(),
                        params.iter().map(|(k, v)| (k, v)),
                        &effect_source,
                    )
                    .is_ok()
                {
                    return draw_image(video, &effect_source, particle, self.config.rotation_order);
                } else {
                    crate::p4_host::warn_once(
                        format!("effect-{}", self.config.particle_effect_name),
                        format!(
                            "パーティクル(R): 粒子別効果 '{}' を実行できません",
                            self.config.particle_effect_name
                        ),
                    );
                }
            }
        }
        draw_image(video, source, particle, self.config.rotation_order)
    }

    fn draw_text_cell(
        &self,
        video: &mut FilterProcVideo<()>,
        particle: ParticleSample,
    ) -> AnyResult<()> {
        let lines: Vec<Vec<char>> = self
            .config
            .source_text
            .lines()
            .map(|line| line.chars().collect())
            .collect();
        if lines.is_empty() {
            return self.draw_image(video, &self.source, particle);
        }
        let (width, height) = match video.get_image_resource_size(&self.source) {
            Ok(size) if size.0 > 0 && size.1 > 0 => size,
            _ => return self.draw_image(video, &self.source, particle),
        };
        let rows = lines.len().min(256);
        let columns = lines.iter().map(Vec::len).max().unwrap_or(0).min(256);
        if columns == 0 {
            return Ok(());
        }
        let (row, column, column_count) = if self.config.source_kind == 3 {
            (particle.id as usize % rows, 0, 1)
        } else {
            let cells: Vec<(usize, usize)> = lines
                .iter()
                .take(rows)
                .enumerate()
                .flat_map(|(r, line)| {
                    line.iter()
                        .take(columns)
                        .enumerate()
                        .filter_map(move |(c, ch)| (!ch.is_whitespace()).then_some((r, c)))
                })
                .collect();
            if cells.is_empty() {
                return Ok(());
            }
            let (r, c) = cells[particle.id as usize % cells.len()];
            (r, c, columns)
        };
        let cell_width = width as f32 / column_count as f32;
        let cell_height = height as f32 / rows as f32;
        let bounds = [
            column as f32 / column_count as f32,
            row as f32 / rows as f32,
            (column + 1) as f32 / column_count as f32,
            (row + 1) as f32 / rows as f32,
        ];
        let quad = quad(
            particle,
            cell_width * particle.scale,
            cell_height * particle.scale,
            bounds,
        );
        video.draw_poly(&VertexList::QuadTexture(vec![quad]), Some(&self.source))?;
        Ok(())
    }

    fn draw_solid(
        &self,
        video: &mut FilterProcVideo<()>,
        particle: ParticleSample,
    ) -> AnyResult<()> {
        let size = self.config.solid_size.max(1) as f32 * particle.scale;
        let color = [
            self.config.solid_r,
            self.config.solid_g,
            self.config.solid_b,
        ]
        .map(|v| v.clamp(0, 255) as f32 / 255.0);
        let tris = solid_triangles(
            self.config.solid_shape,
            self.config.solid_divisions,
            self.config.solid_depth as f32 * particle.scale,
            self.config.solid_curve_x as f32 / 100.0,
            self.config.solid_curve_y as f32 / 100.0,
            self.config.rotation_order,
            particle,
            size,
            color,
        );
        video.draw_poly(&VertexList::TriangleColor(tris), None)?;
        Ok(())
    }

    fn draw_glass(
        &self,
        video: &mut FilterProcVideo<()>,
        particle: ParticleSample,
    ) -> AnyResult<()> {
        let Some(source) = self.background.as_ref() else {
            return self.draw_image(video, &self.source, particle);
        };
        let Ok((width, height)) = video.get_image_resource_size(source) else {
            return self.draw_image(video, &self.source, particle);
        };
        if width == 0 || height == 0 {
            return Ok(());
        }
        let size = self.config.solid_size.max(1) as f32 * particle.scale;
        let offset = self.config.glass_offset as f32;
        let u0 = ((particle.x - size * 0.5 + width as f32 * 0.5 + offset) / width as f32)
            .clamp(0.0, 1.0);
        let v0 = ((particle.y - size * 0.5 + height as f32 * 0.5 + offset) / height as f32)
            .clamp(0.0, 1.0);
        let u1 = (u0 + size / width as f32).clamp(0.0, 1.0);
        let v1 = (v0 + size / height as f32).clamp(0.0, 1.0);
        video.draw_poly(
            &VertexList::QuadTexture(vec![quad(particle, size, size, [u0, v0, u1, v1])]),
            Some(source),
        )?;
        Ok(())
    }
}

fn draw_image(
    video: &mut FilterProcVideo<()>,
    source: &ImageResource,
    particle: ParticleSample,
    rotation_order: i32,
) -> AnyResult<()> {
    if rotation_order != 0
        && let Ok((width, height)) = video.get_image_resource_size(source)
        && width > 0
        && height > 0
    {
        let h_width = width as f32 * particle.scale * 0.5;
        let h_height = height as f32 * particle.scale * 0.5;
        let points = [
            [-h_width, -h_height, 0.0],
            [h_width, -h_height, 0.0],
            [h_width, h_height, 0.0],
            [-h_width, h_height, 0.0],
        ];
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let vertices = std::array::from_fn(|index| {
            let [x, y, z] = rotate_point(points[index], particle, rotation_order);
            VertexTexture {
                x: particle.x + x,
                y: particle.y + y,
                z: particle.z + z,
                u: uv[index][0],
                v: uv[index][1],
                a: particle.alpha,
            }
        });
        video.draw_poly(&VertexList::QuadTexture(vec![vertices]), Some(source))?;
        return Ok(());
    }
    video.draw_image(
        source,
        DrawImageParam {
            x: particle.x,
            y: particle.y,
            z: particle.z,
            rx: particle.rx,
            ry: particle.ry,
            rz: particle.rz,
            sx: particle.scale,
            sy: particle.scale,
            sz: particle.scale,
            alpha: particle.alpha,
        },
    )?;
    Ok(())
}

fn rotate_point(mut point: [f32; 3], p: ParticleSample, rotation_order: i32) -> [f32; 3] {
    let (sx, cx) = p.rx.to_radians().sin_cos();
    let (sy, cy) = p.ry.to_radians().sin_cos();
    let (sz, cz) = p.rz.to_radians().sin_cos();
    let apply = |axis: u8, [x, y, z]: [f32; 3]| match axis {
        b'x' => [x, y * cx - z * sx, y * sx + z * cx],
        b'y' => [x * cy + z * sy, y, -x * sy + z * cy],
        _ => [x * cz - y * sz, x * sz + y * cz, z],
    };
    let order: &[u8; 3] = match rotation_order {
        1 => b"zxy",
        2 => b"zyx",
        3 => b"yzx",
        4 => b"yxz",
        5 => b"xyz",
        6 => b"xzy",
        _ => b"xyz",
    };
    for axis in order {
        point = apply(*axis, point);
    }
    point
}

fn effect_params(raw: &str, particle: ParticleSample, now: f64) -> Vec<(String, String)> {
    raw.split(';')
        .take(16)
        .filter_map(|item| {
            let (key, value) = item.split_once('=')?;
            let key = key.trim();
            if key.is_empty() || key.len() > 100 {
                return None;
            }
            let value = value
                .trim()
                .replace("{id}", &particle.id.to_string())
                .replace(
                    "{age_ms}",
                    &((now - particle.birth_time) * 1000.0).round().to_string(),
                )
                .replace(
                    "{birth_ms}",
                    &(particle.birth_time * 1000.0).round().to_string(),
                );
            (value.len() <= 1024).then_some((key.to_string(), value))
        })
        .collect()
}

fn quad(p: ParticleSample, width: f32, height: f32, uv: [f32; 4]) -> [VertexTexture; 4] {
    let angle = p.rz.to_radians();
    let (s, c) = angle.sin_cos();
    let corner = |x: f32, y: f32, u: f32, v: f32| VertexTexture {
        x: p.x + x * c - y * s,
        y: p.y + x * s + y * c,
        z: p.z,
        u,
        v,
        a: p.alpha,
    };
    [
        corner(-width * 0.5, -height * 0.5, uv[0], uv[1]),
        corner(width * 0.5, -height * 0.5, uv[2], uv[1]),
        corner(width * 0.5, height * 0.5, uv[2], uv[3]),
        corner(-width * 0.5, height * 0.5, uv[0], uv[3]),
    ]
}

fn solid_triangles(
    shape: i32,
    divisions: i32,
    depth: f32,
    curve_x: f32,
    curve_y: f32,
    rotation_order: i32,
    p: ParticleSample,
    size: f32,
    color: [f32; 3],
) -> Vec<[VertexColor; 3]> {
    let h = size * 0.5;
    let divisions = divisions.clamp(3, 20) as usize;
    let mut local = Vec::<[[f32; 3]; 3]>::new();
    let mut quad = |a, b, c, d| {
        local.push([a, b, c]);
        local.push([a, c, d]);
    };
    match shape {
        // Type 1: latitude/longitude sphere. The original uses the particle
        // image on each face; the beta renderer preserves the geometry and
        // uses the configured solid colour.
        1 => {
            for latitude in 0..divisions {
                let v0 = latitude as f32 / divisions as f32;
                let v1 = (latitude + 1) as f32 / divisions as f32;
                let phi0 = std::f32::consts::PI * (v0 - 0.5);
                let phi1 = std::f32::consts::PI * (v1 - 0.5);
                for longitude in 0..divisions * 2 {
                    let u0 = longitude as f32 / (divisions * 2) as f32;
                    let u1 = (longitude + 1) as f32 / (divisions * 2) as f32;
                    let point = |phi: f32, u: f32| {
                        let theta = u * std::f32::consts::TAU;
                        [
                            h * phi.cos() * theta.sin(),
                            h * phi.sin(),
                            h * phi.cos() * theta.cos(),
                        ]
                    };
                    quad(
                        point(phi0, u0),
                        point(phi0, u1),
                        point(phi1, u1),
                        point(phi1, u0),
                    );
                }
            }
        }
        // Type 2/3: cone and bicone.
        2 | 3 => {
            let bottom = if shape == 3 {
                [0.0, h, 0.0]
            } else {
                [0.0, 0.0, 0.0]
            };
            for index in 0..divisions {
                let a0 = index as f32 * std::f32::consts::TAU / divisions as f32;
                let a1 = (index + 1) as f32 * std::f32::consts::TAU / divisions as f32;
                let r0 = [h * a0.sin(), 0.0, h * a0.cos()];
                let r1 = [h * a1.sin(), 0.0, h * a1.cos()];
                local.push([[0.0, -h, 0.0], r1, r0]);
                if shape == 2 {
                    local.push([bottom, r0, r1]);
                } else {
                    local.push([[0.0, h, 0.0], r0, r1]);
                }
            }
        }
        // Type 4: a subdivided sheet curved in both axes.
        4 => {
            let point = |x_index: usize, y_index: usize| {
                let x = x_index as f32 / divisions as f32 * size - h;
                let y = y_index as f32 / divisions as f32 * size - h;
                let nx = if h > 0.0 { x / h } else { 0.0 };
                let ny = if h > 0.0 { y / h } else { 0.0 };
                let z = h * 0.5 * (curve_x * nx * nx + curve_y * ny * ny);
                [x, y, z]
            };
            for y in 0..divisions {
                for x in 0..divisions {
                    quad(
                        point(x, y),
                        point(x + 1, y),
                        point(x + 1, y + 1),
                        point(x, y + 1),
                    );
                }
            }
        }
        // Type 0 is a regular hexahedron; type 5 is the legacy depth form.
        _ => {
            let hz = if shape == 5 {
                depth.abs().max(1.0) * 0.5
            } else {
                h
            };
            let v = [
                [-h, -h, -hz],
                [h, -h, -hz],
                [h, h, -hz],
                [-h, h, -hz],
                [-h, -h, hz],
                [h, -h, hz],
                [h, h, hz],
                [-h, h, hz],
            ];
            for [a, b, c, d] in [
                [0, 3, 2, 1],
                [4, 5, 6, 7],
                [0, 1, 5, 4],
                [1, 2, 6, 5],
                [2, 3, 7, 6],
                [3, 0, 4, 7],
            ] {
                quad(v[a], v[b], v[c], v[d]);
            }
        }
    }
    local
        .into_iter()
        .map(|triangle| {
            triangle.map(|point| {
                let [x, y, z] = rotate_point(point, p, rotation_order);
                VertexColor {
                    x: p.x + x,
                    y: p.y + y,
                    z: p.z + z,
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: p.alpha,
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbered_paths_are_bounded() {
        assert_eq!(
            sequence_path("a/####.png", 42).unwrap(),
            PathBuf::from("a/0042.png")
        );
        assert!(sequence_path("a/fixed.png", 42).is_none());
    }

    #[test]
    fn legacy_solid_types_build_bounded_geometry() {
        let particle = ParticleSample {
            id: 0,
            birth_time: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            rx: 10.0,
            ry: 20.0,
            rz: 30.0,
            scale: 1.0,
            alpha: 1.0,
        };
        for shape in 0..=5 {
            let triangles =
                solid_triangles(shape, 10, 100.0, 0.5, 0.2, 3, particle, 100.0, [1.0; 3]);
            assert!(!triangles.is_empty(), "shape {shape}");
            assert!(triangles.len() <= 800, "shape {shape}");
            assert!(triangles.iter().flatten().all(|vertex| {
                [vertex.x, vertex.y, vertex.z, vertex.a]
                    .into_iter()
                    .all(f32::is_finite)
            }));
        }
    }
}
