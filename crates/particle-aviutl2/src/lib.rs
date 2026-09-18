use aviutl2::{
    AnyResult,
    filter::{
        DrawImageParam, FilterConfigItem, FilterConfigItemSliceExt, FilterConfigItems,
        FilterConfigTrack, FilterConfigTrackGroup, FilterPlugin, FilterPluginTable,
        FilterProcVideo, ImageResource, VertexColor, VertexList,
    },
};
use particle_core::{
    Arrival, EmitterShape, OrbitPlane, P3Config, ParticleConfig, TrailMode, Wave, render_batch,
};

#[aviutl2::filter::filter_config_items]
#[derive(Debug, Clone)]
struct FilterConfig {
    #[track(name = "出力速度", range = 0..=10000, step = 1.0, default = 100, group = "発生")]
    speed: i32,
    #[track(name = "出力頻度", range = 0..=50000, step = 1.0, default = 100, group = "発生")]
    frequency: i32,
    #[track(name = "出力方向", range = -360..=360, step = 1.0, default = 0, group = "発生")]
    direction: i32,
    #[track(name = "拡散角度", range = 0..=180, step = 1.0, default = 60, group = "発生")]
    spread: i32,
    #[track(name = "Z出力方向", range = -360..=360, step = 1.0, default = 0, group = "発生")]
    direction_z: i32,
    #[track(name = "Z拡散角度", range = 0..=180, step = 1.0, default = 0, group = "発生")]
    spread_z: i32,
    #[track(name = "同時発生数", range = 1..=100, step = 1.0, default = 1, group = "発生")]
    simultaneous: i32,
    #[track(name = "発生形状 0点 1線 2箱 3球", range = 0..=3, step = 1.0, default = 0, group = "発生")]
    shape: i32,
    #[track(name = "範囲X", range = 0..=2000, step = 1.0, default = 100, group = "発生")]
    extent_x: i32,
    #[track(name = "範囲Y", range = 0..=2000, step = 1.0, default = 100, group = "発生")]
    extent_y: i32,
    #[track(name = "範囲Z", range = 0..=2000, step = 1.0, default = 100, group = "発生")]
    extent_z: i32,
    #[track(name = "生存時間 1/100秒", range = 1..=6000, step = 1.0, default = 300, group = "時間")]
    lifetime_centis: i32,
    #[track(name = "開始時間 1/100秒", range = 0..=6000, step = 1.0, default = 0, group = "時間")]
    start_centis: i32,
    #[track(name = "重力X", range = -10000..=10000, step = 1.0, default = 0, group = "運動")]
    gravity_x: i32,
    #[track(name = "重力Y", range = -10000..=10000, step = 1.0, default = 0, group = "運動")]
    gravity_y: i32,
    #[track(name = "重力Z", range = -10000..=10000, step = 1.0, default = 0, group = "運動")]
    gravity_z: i32,
    #[track(name = "回転速度Z 度/秒", range = -3600..=3600, step = 1.0, default = 60, group = "運動")]
    rotation_z_speed: i32,
    #[track(name = "開始透過率 %", range = 0..=100, step = 1.0, default = 100, group = "表示")]
    alpha_start: i32,
    #[track(name = "終了透過率 %", range = 0..=100, step = 1.0, default = 50, group = "表示")]
    alpha_end: i32,
    #[track(name = "開始拡大率 %", range = 0..=1000, step = 1.0, default = 100, group = "表示")]
    zoom_start: i32,
    #[track(name = "終了拡大率 %", range = 0..=1000, step = 1.0, default = 100, group = "表示")]
    zoom_end: i32,
    #[track(name = "シード", range = -100000..=100000, step = 1.0, default = 0, group = "その他")]
    seed: i32,

    #[track(name = "風X 開始", range = -10000..=10000, step = 1.0, default = 0, group = "P3 風")]
    wind_from_x: i32,
    #[track(name = "風Y 開始", range = -10000..=10000, step = 1.0, default = 0, group = "P3 風")]
    wind_from_y: i32,
    #[track(name = "風Z 開始", range = -10000..=10000, step = 1.0, default = 0, group = "P3 風")]
    wind_from_z: i32,
    #[track(name = "風X 終了", range = -10000..=10000, step = 1.0, default = 0, group = "P3 風")]
    wind_to_x: i32,
    #[track(name = "風Y 終了", range = -10000..=10000, step = 1.0, default = 0, group = "P3 風")]
    wind_to_y: i32,
    #[track(name = "風Z 終了", range = -10000..=10000, step = 1.0, default = 0, group = "P3 風")]
    wind_to_z: i32,
    #[track(name = "風の変化時間 ms", range = 0..=90000, step = 1.0, default = 0, group = "P3 風")]
    wind_duration_ms: i32,
    #[track(name = "空気抵抗 %/秒", range = 0..=500, step = 1.0, default = 0, group = "P3 風")]
    wind_drag_pct: i32,

    #[track(name = "個別速度ばらつき %", range = 0..=100, step = 1.0, default = 0, group = "P3 個別")]
    variation_speed: i32,
    #[track(name = "個別寿命ばらつき %", range = 0..=90, step = 1.0, default = 0, group = "P3 個別")]
    variation_lifetime: i32,
    #[track(name = "個別回転ばらつき %", range = 0..=100, step = 1.0, default = 0, group = "P3 個別")]
    variation_rotation: i32,
    #[track(name = "個別重力ばらつき %", range = 0..=100, step = 1.0, default = 0, group = "P3 個別")]
    variation_gravity: i32,
    #[track(name = "個別透過率ばらつき %", range = 0..=100, step = 1.0, default = 0, group = "P3 個別")]
    variation_alpha: i32,
    #[track(name = "個別拡大率ばらつき %", range = 0..=100, step = 1.0, default = 0, group = "P3 個別")]
    variation_zoom: i32,

    #[track(name = "頻度変調 %", range = 0..=90, step = 1.0, default = 0, group = "P3 時間")]
    wave_frequency: i32,
    #[track(name = "頻度周期 ms", range = 100..=90000, step = 1.0, default = 1000, group = "P3 時間")]
    wave_frequency_period: i32,
    #[track(name = "速度変調 %", range = 0..=100, step = 1.0, default = 0, group = "P3 時間")]
    wave_speed: i32,
    #[track(name = "速度周期 ms", range = 100..=90000, step = 1.0, default = 1000, group = "P3 時間")]
    wave_speed_period: i32,
    #[track(name = "透過率変調 %", range = 0..=100, step = 1.0, default = 0, group = "P3 時間")]
    wave_alpha: i32,
    #[track(name = "透過率周期 ms", range = 100..=90000, step = 1.0, default = 1000, group = "P3 時間")]
    wave_alpha_period: i32,
    #[track(name = "拡大率変調 %", range = 0..=100, step = 1.0, default = 0, group = "P3 時間")]
    wave_zoom: i32,
    #[track(name = "拡大率周期 ms", range = 100..=90000, step = 1.0, default = 1000, group = "P3 時間")]
    wave_zoom_period: i32,
    #[track(name = "進行速度 %", range = 0..=400, step = 1.0, default = 100, group = "P3 時間")]
    time_scale_pct: i32,
    #[track(name = "進行時間ずれ ms", range = -90000..=90000, step = 1.0, default = 0, group = "P3 時間")]
    time_offset_ms: i32,

    #[track(name = "集結を使用", range = 0..=1, step = 1.0, default = 0, group = "P3 集結")]
    converge_enabled: i32,
    #[track(name = "集結点X", range = -8000..=8000, step = 1.0, default = 0, group = "P3 集結")]
    converge_x: i32,
    #[track(name = "集結点Y", range = -8000..=8000, step = 1.0, default = 0, group = "P3 集結")]
    converge_y: i32,
    #[track(name = "集結点Z", range = -8000..=8000, step = 1.0, default = 0, group = "P3 集結")]
    converge_z: i32,
    #[track(name = "集結力", range = 0..=10000, step = 1.0, default = 0, group = "P3 集結")]
    converge_strength: i32,
    #[track(name = "集結開始 ms", range = 0..=90000, step = 1.0, default = 0, group = "P3 集結")]
    converge_start_ms: i32,
    #[track(name = "到着半径", range = 0..=2000, step = 1.0, default = 0, group = "P3 集結")]
    converge_radius: i32,
    #[track(name = "到着後 0継続 1停止 2消失", range = 0..=2, step = 1.0, default = 0, group = "P3 集結")]
    converge_arrival: i32,

    #[track(name = "境界反射を使用", range = 0..=1, step = 1.0, default = 0, group = "P3 反射")]
    bounce_enabled: i32,
    #[track(name = "X最小", range = -10000..=10000, step = 1.0, default = -640, group = "P3 反射")]
    bounce_x_min: i32,
    #[track(name = "X最大", range = -10000..=10000, step = 1.0, default = 640, group = "P3 反射")]
    bounce_x_max: i32,
    #[track(name = "Y最小", range = -10000..=10000, step = 1.0, default = -360, group = "P3 反射")]
    bounce_y_min: i32,
    #[track(name = "Y最大", range = -10000..=10000, step = 1.0, default = 360, group = "P3 反射")]
    bounce_y_max: i32,
    #[track(name = "Z最小", range = -10000..=10000, step = 1.0, default = -500, group = "P3 反射")]
    bounce_z_min: i32,
    #[track(name = "Z最大", range = -10000..=10000, step = 1.0, default = 500, group = "P3 反射")]
    bounce_z_max: i32,
    #[track(name = "反発係数 %", range = 0..=200, step = 1.0, default = 90, group = "P3 反射")]
    bounce_restitution: i32,

    #[track(name = "分散を使用", range = 0..=1, step = 1.0, default = 0, group = "P3 分散")]
    disperse_enabled: i32,
    #[track(name = "分散時間 ms", range = 0..=90000, step = 1.0, default = 1000, group = "P3 分散")]
    disperse_after_ms: i32,
    #[track(name = "分散速度", range = 0..=10000, step = 1.0, default = 0, group = "P3 分散")]
    disperse_impulse: i32,
    #[track(name = "分散XY角度", range = 0..=180, step = 1.0, default = 0, group = "P3 分散")]
    disperse_xy: i32,
    #[track(name = "分散Z角度", range = 0..=180, step = 1.0, default = 0, group = "P3 分散")]
    disperse_z: i32,
    #[track(name = "反射で分散", range = 0..=1, step = 1.0, default = 0, group = "P3 分散")]
    disperse_on_bounce: i32,
    #[track(name = "急停止時間 ms (0無効)", range = 0..=90000, step = 1.0, default = 0, group = "P3 分散")]
    stop_after_ms: i32,

    #[track(name = "円運動 0無 1XY 2XZ 3YZ 4球", range = 0..=4, step = 1.0, default = 0, group = "P3 円運動")]
    orbit_mode: i32,
    #[track(name = "中心X", range = -8000..=8000, step = 1.0, default = 0, group = "P3 円運動")]
    orbit_x: i32,
    #[track(name = "中心Y", range = -8000..=8000, step = 1.0, default = 0, group = "P3 円運動")]
    orbit_y: i32,
    #[track(name = "中心Z", range = -8000..=8000, step = 1.0, default = 0, group = "P3 円運動")]
    orbit_z: i32,
    #[track(name = "円運動半径", range = -8000..=8000, step = 1.0, default = 0, group = "P3 円運動")]
    orbit_radius: i32,
    #[track(name = "半径速度", range = -1000..=1000, step = 1.0, default = 0, group = "P3 円運動")]
    orbit_radial_speed: i32,
    #[track(name = "角速度 度/秒", range = -1000..=1000, step = 1.0, default = 60, group = "P3 円運動")]
    orbit_angular_speed: i32,
    #[track(name = "球面緯度速度 度/秒", range = -1000..=1000, step = 1.0, default = 0, group = "P3 円運動")]
    orbit_elevation_speed: i32,

    #[track(name = "回転加速度X", range = -3600..=3600, step = 1.0, default = 0, group = "P3 回転")]
    rotation_accel_x: i32,
    #[track(name = "回転加速度Y", range = -3600..=3600, step = 1.0, default = 0, group = "P3 回転")]
    rotation_accel_y: i32,
    #[track(name = "回転加速度Z", range = -3600..=3600, step = 1.0, default = 0, group = "P3 回転")]
    rotation_accel_z: i32,

    #[track(name = "軌跡 0無 1残像 2帯 3点", range = 0..=3, step = 1.0, default = 0, group = "P3 軌跡")]
    trail_mode: i32,
    #[track(name = "軌跡長 ms", range = 1..=5000, step = 1.0, default = 100, group = "P3 軌跡")]
    trail_length_ms: i32,
    #[track(name = "軌跡サンプル数", range = 1..=16, step = 1.0, default = 2, group = "P3 軌跡")]
    trail_samples: i32,
    #[track(name = "軌跡透過率 %", range = 0..=100, step = 1.0, default = 30, group = "P3 軌跡")]
    trail_opacity: i32,
    #[track(name = "軌跡拡大率 %", range = 0..=100, step = 1.0, default = 30, group = "P3 軌跡")]
    trail_scale: i32,
    #[track(name = "帯の幅", range = 1..=100, step = 1.0, default = 10, group = "P3 軌跡")]
    trail_width: i32,
}

impl FilterConfig {
    fn to_core(&self, layer: u32) -> ParticleConfig {
        let mut p3 = P3Config::default();
        p3.wind.from = [
            self.wind_from_x as f64,
            self.wind_from_y as f64,
            self.wind_from_z as f64,
        ];
        p3.wind.to = [
            self.wind_to_x as f64,
            self.wind_to_y as f64,
            self.wind_to_z as f64,
        ];
        p3.wind.duration = self.wind_duration_ms as f64 / 1000.0;
        p3.wind.drag = self.wind_drag_pct as f64 / 100.0;
        p3.variation.speed_percent = self.variation_speed as f64;
        p3.variation.lifetime_percent = self.variation_lifetime as f64;
        p3.variation.rotation_percent = self.variation_rotation as f64;
        p3.variation.gravity_percent = self.variation_gravity as f64;
        p3.variation.alpha_percent = self.variation_alpha as f64;
        p3.variation.zoom_percent = self.variation_zoom as f64;
        p3.modulation.frequency = wave(self.wave_frequency, self.wave_frequency_period);
        p3.modulation.speed = wave(self.wave_speed, self.wave_speed_period);
        p3.modulation.alpha = wave(self.wave_alpha, self.wave_alpha_period);
        p3.modulation.zoom = wave(self.wave_zoom, self.wave_zoom_period);
        p3.time_warp.scale = self.time_scale_pct as f64 / 100.0;
        p3.time_warp.offset = self.time_offset_ms as f64 / 1000.0;
        p3.convergence.enabled = self.converge_enabled != 0;
        p3.convergence.target = [
            self.converge_x as f64,
            self.converge_y as f64,
            self.converge_z as f64,
        ];
        p3.convergence.strength = self.converge_strength as f64;
        p3.convergence.start = self.converge_start_ms as f64 / 1000.0;
        p3.convergence.radius = self.converge_radius as f64;
        p3.convergence.arrival = match self.converge_arrival {
            1 => Arrival::Stop,
            2 => Arrival::Vanish,
            _ => Arrival::Continue,
        };
        p3.boundary.enabled = self.bounce_enabled != 0;
        p3.boundary.min = [
            self.bounce_x_min as f64,
            self.bounce_y_min as f64,
            self.bounce_z_min as f64,
        ];
        p3.boundary.max = [
            self.bounce_x_max as f64,
            self.bounce_y_max as f64,
            self.bounce_z_max as f64,
        ];
        p3.boundary.restitution = self.bounce_restitution as f64 / 100.0;
        p3.dispersion.enabled = self.disperse_enabled != 0;
        p3.dispersion.after = self.disperse_after_ms as f64 / 1000.0;
        p3.dispersion.impulse = self.disperse_impulse as f64;
        p3.dispersion.xy_spread_degrees = self.disperse_xy as f64;
        p3.dispersion.z_spread_degrees = self.disperse_z as f64;
        p3.dispersion.on_bounce = self.disperse_on_bounce != 0;
        p3.dispersion.stop_after = self.stop_after_ms as f64 / 1000.0;
        p3.orbit.plane = match self.orbit_mode {
            1 => OrbitPlane::Xy,
            2 => OrbitPlane::Xz,
            3 => OrbitPlane::Yz,
            4 => OrbitPlane::Sphere,
            _ => OrbitPlane::Off,
        };
        p3.orbit.center = [
            self.orbit_x as f64,
            self.orbit_y as f64,
            self.orbit_z as f64,
        ];
        p3.orbit.radius = self.orbit_radius as f64;
        p3.orbit.radial_speed = self.orbit_radial_speed as f64;
        p3.orbit.angular_speed_degrees = self.orbit_angular_speed as f64;
        p3.orbit.elevation_speed_degrees = self.orbit_elevation_speed as f64;
        p3.rotation_acceleration = [
            self.rotation_accel_x as f64,
            self.rotation_accel_y as f64,
            self.rotation_accel_z as f64,
        ];
        p3.trail.mode = match self.trail_mode {
            1 => TrailMode::Afterimage,
            2 => TrailMode::Ribbon,
            3 => TrailMode::Points,
            _ => TrailMode::Off,
        };
        p3.trail.length = self.trail_length_ms as f64 / 1000.0;
        p3.trail.samples = self.trail_samples.max(0) as u32;
        p3.trail.opacity = self.trail_opacity as f64 / 100.0;
        p3.trail.scale = self.trail_scale as f64 / 100.0;
        p3.trail.width = self.trail_width as f64;
        ParticleConfig {
            speed: self.speed as f64,
            frequency: self.frequency as f64,
            direction_degrees: self.direction as f64,
            spread_degrees: self.spread as f64,
            direction_z_degrees: self.direction_z as f64,
            spread_z_degrees: self.spread_z as f64,
            rotation_z_degrees_per_second: self.rotation_z_speed as f64,
            simultaneous: self.simultaneous.max(1) as u32,
            lifetime: self.lifetime_centis.max(1) as f64 / 100.0,
            start_time: self.start_centis.max(0) as f64 / 100.0,
            gravity_x: self.gravity_x as f64,
            gravity_y: self.gravity_y as f64,
            gravity_z: self.gravity_z as f64,
            alpha_start: self.alpha_start as f64 / 100.0,
            alpha_end: self.alpha_end as f64 / 100.0,
            zoom_start: self.zoom_start as f64 / 100.0,
            zoom_end: self.zoom_end as f64 / 100.0,
            shape: match self.shape {
                1 => EmitterShape::Line,
                2 => EmitterShape::Box,
                3 => EmitterShape::Sphere,
                _ => EmitterShape::Point,
            },
            extent_x: self.extent_x as f64,
            extent_y: self.extent_y as f64,
            extent_z: self.extent_z as f64,
            seed: self.seed,
            layer,
            p3,
        }
    }
}

fn wave(depth_percent: i32, period_ms: i32) -> Wave {
    Wave {
        depth: depth_percent as f64 / 100.0,
        period: period_ms.max(1) as f64 / 1000.0,
    }
}

// FilterConfigItem itself is not Send because it also supports opaque data
// items. This filter only declares tracks and track groups, which are Send.
enum TrackItem {
    Track(FilterConfigTrack),
    Group(FilterConfigTrackGroup),
}

fn build_config_items() -> Vec<FilterConfigItem> {
    let tracks = std::thread::Builder::new()
        .name("particle-config-items".to_string())
        .stack_size(4 * 1024 * 1024)
        .spawn(|| {
            FilterConfig::to_config_items()
                .into_iter()
                .map(|item| match item {
                    FilterConfigItem::Track(track) => TrackItem::Track(track),
                    FilterConfigItem::TrackGroup(group) => TrackItem::Group(group),
                    _ => panic!("unexpected filter config item"),
                })
                .collect::<Vec<_>>()
        })
        .expect("failed to create config metadata thread")
        .join()
        .expect("failed to build config metadata");
    tracks
        .into_iter()
        .map(|item| match item {
            TrackItem::Track(track) => FilterConfigItem::Track(track),
            TrackItem::Group(group) => FilterConfigItem::TrackGroup(group),
        })
        .collect()
}

#[aviutl2::plugin(FilterPlugin)]
struct ParticleFilter;

impl FilterPlugin for ParticleFilter {
    type Userdata = ();

    fn new(_info: aviutl2::AviUtl2Info) -> AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> FilterPluginTable {
        // The derive macro builds every track and group in one expression.
        // A dedicated stack avoids overflowing AviUtl2's initialization thread.
        let config_items = build_config_items();
        FilterPluginTable {
            name: "パーティクル(R) 基本版".to_string(),
            label: None,
            information: format!(
                "Particle (R) Rust port with P3 motion v{}",
                env!("CARGO_PKG_VERSION")
            ),
            flags: aviutl2::bitflag!(aviutl2::filter::FilterPluginFlags { video: true }),
            config_items,
        }
    }

    fn proc_video(
        &self,
        config: &[aviutl2::filter::FilterConfigItem],
        video: &mut FilterProcVideo<Self::Userdata>,
    ) -> AnyResult<()> {
        let config: FilterConfig = config.to_struct();
        let batch = render_batch(&config.to_core(video.object.layer), video.object.time);
        let mut quads = Vec::with_capacity(batch.trail_segments.len());
        for segment in batch.trail_segments {
            let dx = segment.to[0] - segment.from[0];
            let dy = segment.to[1] - segment.from[1];
            let length = dx.hypot(dy);
            if length <= 1e-6 {
                continue;
            }
            let nx = -dy / length * segment.width * 0.5;
            let ny = dx / length * segment.width * 0.5;
            let color = |x, y, z| VertexColor {
                x,
                y,
                z,
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: segment.alpha,
            };
            quads.push([
                color(segment.from[0] + nx, segment.from[1] + ny, segment.from[2]),
                color(segment.to[0] + nx, segment.to[1] + ny, segment.to[2]),
                color(segment.to[0] - nx, segment.to[1] - ny, segment.to[2]),
                color(segment.from[0] - nx, segment.from[1] - ny, segment.from[2]),
            ]);
        }
        if !quads.is_empty() {
            video.draw_poly(&VertexList::QuadColor(quads), None)?;
        }
        for particle in batch.trail_images {
            draw_particle(video, particle)?;
        }
        for particle in batch.particles {
            draw_particle(video, particle)?;
        }
        video.prevent_post_effect();
        Ok(())
    }
}

fn draw_particle(
    video: &mut FilterProcVideo<()>,
    particle: particle_core::ParticleSample,
) -> AnyResult<()> {
    video.draw_image(
        &ImageResource::Object,
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

aviutl2::register_filter_plugin!(ParticleFilter);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_metadata_builds_on_a_one_megabyte_host_stack() {
        let count = std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(|| ParticleFilter.plugin_info().config_items.len())
            .unwrap()
            .join()
            .unwrap();
        assert!(count > 20);
    }
}
