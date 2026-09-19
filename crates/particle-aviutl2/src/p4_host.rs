//! Host reads for P4 links. All calls stay on AviUtl2's render thread.

use aviutl2::filter::{FilterProcVideo, ImageResource};
use particle_core::{ParticleConfig, ParticleSample, RenderBatch, Wave, WindCurve};
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

use crate::FilterConfig;

const FILTER_NAME: &str = "パーティクル(R) 基本版";

pub(super) fn warn_once(key: String, message: String) {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warnings = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut seen) = warnings.lock() {
        if seen.len() < 128 && seen.insert(key) {
            let _ = aviutl2::logger::write_warn_log(&message);
        }
    }
}

pub(super) fn prepare_source(video: &mut FilterProcVideo<()>, layer: i32) -> ImageResource {
    if layer < 0 || layer as u32 == video.object.layer {
        return ImageResource::Object;
    }
    let layer = layer as u32;
    if video.get_image_object(layer, 0.0).is_none() {
        warn_once(
            format!("source-{layer}"),
            format!("パーティクル(R): 素材レイヤー {layer} が見つからないため現在画像を使用します"),
        );
        return ImageResource::Object;
    }
    // draw_image does not accept ImageResource::Layer. Copy the base image into
    // a resource accepted by draw_image. Skip additional effects to avoid a
    // render dependency cycle when two effects point to one another.
    let source = ImageResource::Layer {
        layer,
        apply_additional_effects: false,
    };
    let prepared = ImageResource::Resource(format!("particle2r-source-{}", video.object.id));
    if video.copy_image_resource(&source, &prepared).is_ok() {
        prepared
    } else {
        warn_once(
            format!("source-copy-{layer}"),
            format!("パーティクル(R): 素材レイヤー {layer} の画像を取得できません"),
        );
        ImageResource::Object
    }
}

pub(super) fn apply_shared_settings(
    video: &mut FilterProcVideo<()>,
    ui: &FilterConfig,
    core: &mut ParticleConfig,
) {
    if let Some(wind) = read_linked_tracks(
        video,
        ui.shared_wind_layer,
        &[FILTER_NAME, "パーティクル(R) 風"],
        &[
            "風X 開始",
            "風Y 開始",
            "風Z 開始",
            "風X 終了",
            "風Y 終了",
            "風Z 終了",
            "風の変化時間 ms",
            "空気抵抗 %/秒",
        ],
    ) {
        core.p3.wind = WindCurve {
            from: [wind[0], wind[1], wind[2]],
            to: [wind[3], wind[4], wind[5]],
            duration: wind[6].max(0.0) / 1000.0,
            drag: wind[7].max(0.0) / 100.0,
        };
    }
    if let Some(values) = read_linked_tracks(
        video,
        ui.shared_time_layer,
        &[FILTER_NAME],
        &[
            "進行速度 %",
            "進行時間ずれ ms",
            "頻度変調 %",
            "頻度周期 ms",
            "速度変調 %",
            "速度周期 ms",
            "透過率変調 %",
            "透過率周期 ms",
            "拡大率変調 %",
            "拡大率周期 ms",
        ],
    ) {
        core.p3.time_warp.scale = values[0].max(0.0) / 100.0;
        core.p3.time_warp.offset = values[1] / 1000.0;
        core.p3.modulation.frequency = wave(values[2], values[3]);
        core.p3.modulation.speed = wave(values[4], values[5]);
        core.p3.modulation.alpha = wave(values[6], values[7]);
        core.p3.modulation.zoom = wave(values[8], values[9]);
    } else {
        if let Some(values) = read_linked_tracks(
            video,
            ui.shared_time_layer,
            &["パーティクル(R) 時間2"],
            &["進行速度 %", "進行時間ずれ ms"],
        ) {
            core.p3.time_warp.scale = values[0].max(0.0) / 100.0;
            core.p3.time_warp.offset = values[1] / 1000.0;
        }
        if let Some(values) = read_linked_tracks(
            video,
            ui.shared_time_layer,
            &["パーティクル(R) 時間"],
            &[
                "頻度変調 %",
                "頻度周期 ms",
                "速度変調 %",
                "速度周期 ms",
                "透過率変調 %",
                "透過率周期 ms",
                "拡大率変調 %",
                "拡大率周期 ms",
            ],
        ) {
            core.p3.modulation.frequency = wave(values[0], values[1]);
            core.p3.modulation.speed = wave(values[2], values[3]);
            core.p3.modulation.alpha = wave(values[4], values[5]);
            core.p3.modulation.zoom = wave(values[6], values[7]);
        }
    }
}

fn wave(depth_percent: f64, period_ms: f64) -> Wave {
    Wave {
        depth: depth_percent / 100.0,
        period: period_ms.max(1.0) / 1000.0,
    }
}

fn read_linked_tracks(
    video: &mut FilterProcVideo<()>,
    layer: i32,
    effect_names: &[&str],
    names: &[&str],
) -> Option<Vec<f64>> {
    if layer < 0 || layer as u32 == video.object.layer {
        return None;
    }
    let object = match video.get_image_object(layer as u32, 0.0) {
        Some(object) => object,
        None => {
            warn_once(
                format!("shared-{layer}"),
                format!("パーティクル(R): 共有先レイヤー {layer} が見つかりません"),
            );
            return None;
        }
    };
    let scene_frame = video.object.frame_s as usize + video.object.frame as usize;
    let section = video.read_section();
    let range = section.get_object_layer_frame(object).ok()?;
    if !range.frame_range_inclusive().contains(&scene_frame) {
        return None;
    }
    let target_frame = (scene_frame - range.start) as f64;
    for effect in section.get_effects(object).ok()? {
        if !effect_names.contains(&section.get_effect_name(effect).ok()?.as_str())
            || !section.get_effect_enable(effect).ok()?
        {
            continue;
        }
        let values = names
            .iter()
            .map(|name| {
                section
                    .get_effect_track_value(effect, name, target_frame)
                    .ok()
            })
            .collect::<Option<Vec<_>>>()?;
        if values.iter().all(|value| value.is_finite()) {
            return Some(values);
        }
    }
    None
}

pub(super) fn tracking_delta(
    video: &mut FilterProcVideo<()>,
    layer: i32,
    birth_time: Option<f64>,
) -> Option<[f32; 3]> {
    if layer < 0 || layer as u32 == video.object.layer {
        return None;
    }
    let offset = birth_time.map_or(0.0, |birth| birth - video.object.time);
    if !offset.is_finite() {
        return None;
    }
    let target = match video.get_image_object(layer as u32, offset) {
        Some(target) => target,
        None => {
            warn_once(
                format!("tracking-{layer}"),
                format!("パーティクル(R): 追跡レイヤー {layer} が見つかりません"),
            );
            return None;
        }
    };
    let param = video.get_output_image_param(Some(target), offset).ok()?;
    let delta = [
        param.x - video.param.x,
        param.y - video.param.y,
        param.z - video.param.z,
    ];
    delta.iter().all(|value| value.is_finite()).then_some(delta)
}

pub(super) fn offset_sample(particle: &mut ParticleSample, delta: [f32; 3]) {
    particle.x += delta[0];
    particle.y += delta[1];
    particle.z += delta[2];
}

pub(super) fn apply_tracking(
    video: &mut FilterProcVideo<()>,
    ui: &FilterConfig,
    batch: &mut RenderBatch,
) {
    let Some(current) = tracking_delta(video, ui.tracking_layer, None) else {
        return;
    };
    let mut by_birth = HashMap::<u64, [f32; 3]>::new();
    for particle in batch
        .particles
        .iter_mut()
        .chain(batch.trail_images.iter_mut())
    {
        let delta = if ui.tracking_time == 1 && by_birth.len() < 4096 {
            *by_birth
                .entry(particle.birth_time.to_bits())
                .or_insert_with(|| {
                    tracking_delta(video, ui.tracking_layer, Some(particle.birth_time))
                        .unwrap_or(current)
                })
        } else {
            current
        };
        offset_sample(particle, delta);
    }
    for segment in &mut batch.trail_segments {
        for axis in 0..3 {
            segment.from[axis] += current[axis];
            segment.to[axis] += current[axis];
        }
    }
}
