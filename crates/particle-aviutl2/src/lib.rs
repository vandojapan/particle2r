#![cfg_attr(
    any(feature = "stack-basic", feature = "stack-extension"),
    allow(dead_code)
)]

use aviutl2::{
    AnyResult,
    filter::{
        FilterConfigCheck, FilterConfigFile, FilterConfigItem, FilterConfigItemSliceExt,
        FilterConfigItems, FilterConfigString, FilterConfigText, FilterConfigTrack,
        FilterConfigTrackGroup, FilterPlugin, FilterPluginTable, FilterProcVideo, ImageResource,
        OutputImageResourcePixelFormat, VertexColor, VertexList,
    },
};
use particle_core::{
    AlphaMask, Arrival, EmitterShape, MaskClip, MaskCollision, MaskCollisionMode, OrbitPlane,
    P3Config, ParticleConfig, RenderWorkspace, TrailMode, Wave, build_funnel, build_mesh,
    build_mesh_faces, render_batch_cached, render_batch_with_mask_cached,
};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

const PARTICLE_LABEL: &str = "パーティクル(R)";
pub(crate) const PARTICLE_SCRIPT_SUFFIX: &str = "@particle2r";

mod legacy_ui;
mod p4_audio;
mod p4_draw;
mod p4_host;
mod p6_profile;
mod script_control;
mod stack;
#[cfg(feature = "stack-basic")]
use stack::StackBasicFilter;
#[cfg(feature = "stack-extension")]
use stack::StackExtensionFilter;

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
    #[track(name = "初期回転Z 度", range = -360..=360, step = 1.0, default = 0, group = "運動")]
    rotation_z_initial: i32,
    #[track(name = "回転速度Z 度/秒", range = -3600..=3600, step = 1.0, default = 0, group = "運動")]
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
    #[track(name = "素材レイヤー (-1=現在)", range = -1..=100, step = 1.0, default = -1, group = "P4 素材")]
    source_layer: i32,
    #[track(name = "素材種類 0画像 1連番 2文字 3行 4立体 5ガラス 6画像一覧", range = 0..=6, step = 1.0, default = 0, group = "P4 素材")]
    source_kind: i32,
    #[string(name = "連番パス (#を番号に置換)")]
    sequence_pattern: String,
    #[text(name = "画像一覧 (1行1ファイル)")]
    image_files: String,
    #[check(name = "画像一覧をランダム", default = false)]
    image_random: bool,
    #[track(name = "連番fps", range = 1..=240, step = 1.0, default = 30, group = "P4 素材")]
    sequence_fps: i32,
    #[track(name = "素材時刻 0現在 1出生 2年齢", range = 0..=2, step = 1.0, default = 2, group = "P4 素材")]
    source_time: i32,
    #[text(name = "文字・行の分割元テキスト")]
    source_text: String,
    #[track(name = "立体形状 0六面 1球 2錐 3双錐 4曲面 5奥行", range = 0..=5, step = 1.0, default = 0, group = "P4 立体")]
    solid_shape: i32,
    #[track(name = "立体サイズ", range = 1..=1000, step = 1.0, default = 50, group = "P4 立体")]
    solid_size: i32,
    #[track(name = "立体分割数", range = 3..=20, step = 1.0, default = 10, group = "P4 立体")]
    solid_divisions: i32,
    #[track(name = "立体奥行き", range = 0..=2000, step = 1.0, default = 100, group = "P4 立体")]
    solid_depth: i32,
    #[track(name = "立体横曲率", range = -200..=200, step = 1.0, default = 50, group = "P4 立体")]
    solid_curve_x: i32,
    #[track(name = "立体縦曲率", range = -200..=200, step = 1.0, default = 20, group = "P4 立体")]
    solid_curve_y: i32,
    #[track(name = "回転表現 0標準 1-6軸順", range = 0..=6, step = 1.0, default = 0, group = "P4 立体")]
    rotation_order: i32,
    #[track(name = "立体色 R", range = 0..=255, step = 1.0, default = 255, group = "P4 立体")]
    solid_r: i32,
    #[track(name = "立体色 G", range = 0..=255, step = 1.0, default = 255, group = "P4 立体")]
    solid_g: i32,
    #[track(name = "立体色 B", range = 0..=255, step = 1.0, default = 255, group = "P4 立体")]
    solid_b: i32,
    #[track(name = "ガラス屈折距離", range = -100..=100, step = 1.0, default = 12, group = "P4 ガラス")]
    glass_offset: i32,
    #[string(name = "粒子別効果名")]
    particle_effect_name: String,
    #[string(name = "粒子別効果パラメータ key=value;...")]
    particle_effect_params: String,
    #[file(name = "音声WAV (PCM16)", filters = { "WAV" => ["wav"] })]
    audio_file: Option<std::path::PathBuf>,
    #[track(name = "音声帯域 0無効 1-10", range = 0..=10, step = 1.0, default = 0, group = "P4 音声")]
    audio_band: i32,
    #[track(name = "音声速度変調 %", range = 0..=500, step = 1.0, default = 100, group = "P4 音声")]
    audio_speed_depth: i32,
    #[track(name = "音声頻度変調 %", range = 0..=500, step = 1.0, default = 0, group = "P4 音声")]
    audio_frequency_depth: i32,
    #[track(name = "音声透過率変調 %", range = 0..=100, step = 1.0, default = 0, group = "P4 音声")]
    audio_alpha_depth: i32,
    #[track(name = "音声拡大率変調 %", range = 0..=500, step = 1.0, default = 0, group = "P4 音声")]
    audio_zoom_depth: i32,
    #[track(name = "追跡レイヤー (-1=無効)", range = -1..=100, step = 1.0, default = -1, group = "P4 素材")]
    tracking_layer: i32,
    #[check(name = "追跡時刻 0現在 1出生", default = false)]
    tracking_time: bool,
    #[track(name = "共有風レイヤー (-1=無効)", range = -1..=100, step = 1.0, default = -1, group = "P4 共有")]
    shared_wind_layer: i32,
    #[track(name = "共有時間レイヤー (-1=無効)", range = -1..=100, step = 1.0, default = -1, group = "P4 共有")]
    shared_time_layer: i32,
    #[track(name = "マスクレイヤー (-1=無効)", range = -1..=100, step = 1.0, default = -1, group = "P4 マスク")]
    mask_layer: i32,
    #[track(name = "マスク 0無 1内消 2外消 3反射 4停止 5消失", range = 0..=5, step = 1.0, default = 0, group = "P4 マスク")]
    mask_mode: i32,
    #[track(name = "マスク不透明しきい値", range = 1..=255, step = 1.0, default = 1, group = "P4 マスク")]
    mask_threshold: i32,
    #[track(name = "マスク反発係数 %", range = 0..=200, step = 1.0, default = 100, group = "P4 マスク")]
    mask_restitution: i32,
    #[check(name = "メッシュを使用", default = false)]
    mesh_enabled: bool,
    #[track(name = "接続距離", range = 1..=4000, step = 1.0, default = 100, group = "P4 メッシュ")]
    mesh_distance: i32,
    #[track(name = "粒子ごとの最大接続", range = 1..=16, step = 1.0, default = 2, group = "P4 メッシュ")]
    mesh_max_links: i32,
    #[track(name = "線の幅", range = 1..=100, step = 1.0, default = 1, group = "P4 メッシュ")]
    mesh_width: i32,
    #[track(name = "線の透過率 %", range = 0..=100, step = 1.0, default = 50, group = "P4 メッシュ")]
    mesh_alpha: i32,
    #[check(name = "面を描画", default = false)]
    mesh_faces: bool,
    #[track(name = "線色 R", range = 0..=255, step = 1.0, default = 255, group = "P4 メッシュ")]
    mesh_r: i32,
    #[track(name = "線色 G", range = 0..=255, step = 1.0, default = 255, group = "P4 メッシュ")]
    mesh_g: i32,
    #[track(name = "線色 B", range = 0..=255, step = 1.0, default = 255, group = "P4 メッシュ")]
    mesh_b: i32,
    #[track(name = "ファンネル数", range = 0..=128, step = 1.0, default = 0, group = "P4 ファンネル")]
    funnel_count: i32,
    #[track(name = "ファンネルサイズ %", range = 1..=500, step = 1.0, default = 30, group = "P4 ファンネル")]
    funnel_scale: i32,
    #[track(name = "ファンネル半径", range = 1..=1500, step = 1.0, default = 150, group = "P4 ファンネル")]
    funnel_radius: i32,
    #[track(name = "ファンネル公転速度 度/秒", range = -1000..=1000, step = 1.0, default = 80, group = "P4 ファンネル")]
    funnel_angular_speed: i32,
    #[track(name = "ファンネル円環数", range = 1..=8, step = 1.0, default = 1, group = "P4 ファンネル")]
    funnel_rings: i32,
    #[track(name = "ファンネル自転 度/秒", range = -3600..=3600, step = 1.0, default = 0, group = "P4 ファンネル")]
    funnel_self_spin: i32,
    #[text(name = "ファンネル画像一覧 (1行1ファイル)")]
    funnel_image_files: String,
    #[check(name = "ファンネル画像をランダム", default = true)]
    funnel_image_random: bool,

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

    #[check(name = "集結を使用", default = false)]
    converge_enabled: bool,
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

    #[check(name = "境界反射を使用", default = false)]
    bounce_enabled: bool,
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

    #[check(name = "分散を使用", default = false)]
    disperse_enabled: bool,
    #[track(name = "分散時間 ms", range = 0..=90000, step = 1.0, default = 1000, group = "P3 分散")]
    disperse_after_ms: i32,
    #[track(name = "分散速度", range = 0..=10000, step = 1.0, default = 0, group = "P3 分散")]
    disperse_impulse: i32,
    #[track(name = "分散XY角度", range = 0..=180, step = 1.0, default = 0, group = "P3 分散")]
    disperse_xy: i32,
    #[track(name = "分散Z角度", range = 0..=180, step = 1.0, default = 0, group = "P3 分散")]
    disperse_z: i32,
    #[check(name = "反射で分散", default = false)]
    disperse_on_bounce: bool,
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
        p3.convergence.enabled = self.converge_enabled;
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
        p3.boundary.enabled = self.bounce_enabled;
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
        p3.dispersion.enabled = self.disperse_enabled;
        p3.dispersion.after = self.disperse_after_ms as f64 / 1000.0;
        p3.dispersion.impulse = self.disperse_impulse as f64;
        p3.dispersion.xy_spread_degrees = self.disperse_xy as f64;
        p3.dispersion.z_spread_degrees = self.disperse_z as f64;
        p3.dispersion.on_bounce = self.disperse_on_bounce;
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
            initial_rotation_z_degrees: self.rotation_z_initial as f64,
            rotation_z_degrees_per_second: self.rotation_z_speed as f64,
            face_direction: false,
            simultaneous: self.simultaneous.max(1) as u32,
            lifetime: self.lifetime_centis.max(1) as f64 / 100.0,
            start_time: self.start_centis.max(0) as f64 / 100.0,
            end_time: None,
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
            script_motion: Default::default(),
        }
    }
}

fn wave(depth_percent: i32, period_ms: i32) -> Wave {
    Wave {
        depth: depth_percent as f64 / 100.0,
        period: period_ms.max(1) as f64 / 1000.0,
    }
}

const MAX_MASK_PIXELS: usize = 16_777_216;
const MAX_RENDER_WORKSPACES: usize = 16;
static RENDER_WORKSPACES: OnceLock<Mutex<HashMap<String, RenderWorkspace>>> = OnceLock::new();

fn take_render_workspace(object_id: impl ToString) -> (String, RenderWorkspace) {
    let key = object_id.to_string();
    let workspace = RENDER_WORKSPACES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|mut cache| cache.remove(&key))
        .unwrap_or_default();
    (key, workspace)
}

fn return_render_workspace(key: String, workspace: RenderWorkspace) {
    let Ok(mut cache) = RENDER_WORKSPACES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    else {
        return;
    };
    if cache.len() >= MAX_RENDER_WORKSPACES
        && let Some(oldest) = cache.keys().next().cloned()
    {
        cache.remove(&oldest);
    }
    cache.insert(key, workspace);
}

fn read_layer_alpha_mask(video: &mut FilterProcVideo<()>, layer: i32) -> Option<AlphaMask> {
    if layer < 0 || layer as u32 == video.object.layer {
        return None;
    }
    let layer = layer as u32;
    if video.get_image_object(layer, 0.0).is_none() {
        p4_host::warn_once(
            format!("mask-{layer}"),
            format!("パーティクル(R): マスクレイヤー {layer} が見つかりません"),
        );
        return None;
    }
    let source = ImageResource::Layer {
        layer,
        apply_additional_effects: false,
    };
    let cache = ImageResource::Resource(format!("particle2r-mask-{}", video.object.id));
    video.copy_image_resource(&source, &cache).ok()?;
    let (width, height) = video.get_image_resource_size(&cache).ok()?;
    let pixels = (width as usize).checked_mul(height as usize)?;
    if pixels == 0 || pixels > MAX_MASK_PIXELS {
        return None;
    }
    let pitch = width.checked_mul(4)?;
    let mut rgba = vec![0_u8; pixels.checked_mul(4)?];
    video
        .get_image_resource_data(
            &cache,
            &mut rgba,
            width,
            height,
            pitch,
            OutputImageResourcePixelFormat::Rgba,
        )
        .ok()?;
    AlphaMask::from_rgba(width, height, &rgba)
}

fn mask_clip(mode: i32) -> MaskClip {
    match mode {
        1 => MaskClip::HideInside,
        2 => MaskClip::HideOutside,
        _ => MaskClip::Disabled,
    }
}

// FilterConfigItem itself is not Send because it also supports opaque data
// items. The declared tracks, checks, strings, text and files are all Send.
enum TrackItem {
    Track(FilterConfigTrack),
    Check(FilterConfigCheck),
    Group(FilterConfigTrackGroup),
    String(FilterConfigString),
    Text(FilterConfigText),
    File(FilterConfigFile),
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
                    FilterConfigItem::Check(check) => TrackItem::Check(check),
                    FilterConfigItem::TrackGroup(group) => TrackItem::Group(group),
                    FilterConfigItem::String(value) => TrackItem::String(value),
                    FilterConfigItem::Text(value) => TrackItem::Text(value),
                    FilterConfigItem::File(value) => TrackItem::File(value),
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
            TrackItem::Check(check) => FilterConfigItem::Check(check),
            TrackItem::Group(group) => FilterConfigItem::TrackGroup(group),
            TrackItem::String(value) => FilterConfigItem::String(value),
            TrackItem::Text(value) => FilterConfigItem::Text(value),
            TrackItem::File(value) => FilterConfigItem::File(value),
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
            name: format!("パーティクル(R) 基本版{}", PARTICLE_SCRIPT_SUFFIX),
            label: Some(PARTICLE_LABEL.to_string()),
            information: format!(
                "Particle (R) Rust port with P4 host features v{}",
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
        render_filter(&config, video)
    }
}

fn render_filter(config: &FilterConfig, video: &mut FilterProcVideo<()>) -> AnyResult<()> {
    render_filter_scripted(config, video, None, None, RenderOptions::default(), false)
}

#[derive(Clone, Copy)]
struct RenderOptions {
    reverse_draw_order: bool,
    inverse_frequency: bool,
    reverse_time: bool,
    face_direction: bool,
    face_mode: i32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            reverse_draw_order: false,
            inverse_frequency: false,
            reverse_time: false,
            face_direction: false,
            // -1 means that the extension is absent. When it is present its
            // legacy default is 0 (front side only).
            face_mode: -1,
        }
    }
}

fn render_filter_scripted(
    config: &FilterConfig,
    video: &mut FilterProcVideo<()>,
    output_source: Option<&str>,
    behavior_source: Option<&str>,
    options: RenderOptions,
    end_at_object: bool,
) -> AnyResult<()> {
    let profile_started = p6_profile::enabled().then(std::time::Instant::now);
    let mut core = config.to_core(video.object.layer);
    if options.inverse_frequency && core.frequency > 0.0 {
        // Legacy inverse frequency is an interval percentage: 100 means one
        // emission per second and 200 means one emission per two seconds.
        core.frequency = 1000.0 / core.frequency;
    }
    core.face_direction = options.face_direction;
    if end_at_object {
        core.end_time = Some(video.object.time_total);
    }
    p4_host::apply_shared_settings(video, &config, &mut core);
    if output_source.is_some() || behavior_source.is_some() {
        let longest_lifetime =
            core.lifetime * (1.0 + core.p3.variation.lifetime_percent.abs() / 100.0).max(1.0);
        let script_time = if options.reverse_time {
            (video.object.time_total - video.object.time).max(0.0)
        } else {
            video.object.time
        };
        let script_frame = if options.reverse_time {
            video.object.frame_total.saturating_sub(video.object.frame)
        } else {
            video.object.frame
        };
        let uses_getvalue = output_source
            .into_iter()
            .chain(behavior_source)
            .any(|source| source.contains("obj.getvalue"));
        let host_values = uses_getvalue
            .then(|| p4_host::previous_layer_value_trace(video, script_time.max(longest_lifetime)))
            .flatten();
        match script_control::build_motion(
            output_source,
            behavior_source,
            script_time,
            video.object.time_total,
            script_frame,
            video.object.frame_total,
            *video.scene.frame_rate.numer() as f64 / *video.scene.frame_rate.denom() as f64,
            longest_lifetime,
            video.object.layer,
            host_values.as_ref(),
        ) {
            Ok(motion) => core.script_motion = motion,
            Err(error) => p4_host::warn_once(
                format!("script-control-{}", video.object.id),
                format!("パーティクル(R): スクリプト制御: {error}"),
            ),
        }
    }
    if config.audio_band > 0 {
        if let Some(file) = config.audio_file.as_deref() {
            if let Some(level) =
                p4_audio::band_at(file, video.object.time, (config.audio_band - 1) as usize)
            {
                core.speed *=
                    1.0 + level as f64 * config.audio_speed_depth.clamp(0, 500) as f64 / 100.0;
                core.frequency *=
                    1.0 + level as f64 * config.audio_frequency_depth.clamp(0, 500) as f64 / 100.0;
                let alpha_factor =
                    1.0 - level as f64 * config.audio_alpha_depth.clamp(0, 100) as f64 / 100.0;
                core.alpha_start *= alpha_factor;
                core.alpha_end *= alpha_factor;
                let zoom_factor =
                    1.0 + level as f64 * config.audio_zoom_depth.clamp(0, 500) as f64 / 100.0;
                core.zoom_start *= zoom_factor;
                core.zoom_end *= zoom_factor;
            } else {
                p4_host::warn_once(
                    format!("audio-{}", file.display()),
                    format!(
                        "パーティクル(R): 音声を解析できません: {} (PCM16 WAV が必要です)",
                        file.display()
                    ),
                );
            }
        }
    }
    let mask = if config.mask_mode != 0 {
        read_layer_alpha_mask(video, config.mask_layer)
    } else {
        None
    };
    let collision_mode = match config.mask_mode {
        3 => Some(MaskCollisionMode::Bounce),
        4 => Some(MaskCollisionMode::Stop),
        5 => Some(MaskCollisionMode::Vanish),
        _ => None,
    };
    let (workspace_key, mut workspace) = take_render_workspace(video.object.id);
    let render_time = if options.reverse_time {
        (video.object.time_total - video.object.time).max(0.0)
    } else {
        video.object.time
    };
    let mut batch = if let (Some(mask), Some(mode)) = (mask.as_ref(), collision_mode) {
        render_batch_with_mask_cached(
            &core,
            render_time,
            MaskCollision {
                mask,
                threshold: config.mask_threshold.clamp(1, 255) as u8,
                mode,
                restitution: config.mask_restitution.clamp(0, 200) as f64 / 100.0,
            },
            &mut workspace,
        )
    } else {
        render_batch_cached(&core, render_time, &mut workspace)
    };
    return_render_workspace(workspace_key, workspace);
    p4_host::apply_tracking(video, &config, &mut batch);
    if options.reverse_draw_order {
        batch.particles.reverse();
        batch.trail_images.reverse();
    }
    if (0..=2).contains(&options.face_mode) {
        let keep = |particle: &particle_core::ParticleSample| {
            let normal_z = particle.rx.to_radians().cos() * particle.ry.to_radians().cos();
            if options.face_mode == 0 {
                normal_z >= 0.0
            } else {
                normal_z < 0.0
            }
        };
        batch.particles.retain(keep);
        batch.trail_images.retain(keep);
        if options.face_mode == 1 {
            for particle in batch
                .particles
                .iter_mut()
                .chain(batch.trail_images.iter_mut())
            {
                particle.ry += 180.0;
            }
        }
    }
    let source = p4_host::prepare_source(video, config.source_layer);
    let background = if config.source_kind == 5 {
        let snapshot =
            ImageResource::Resource(format!("particle2r-background-{}", video.object.id));
        video
            .copy_image_resource(&ImageResource::Framebuffer, &snapshot)
            .ok()
            .map(|_| snapshot)
    } else {
        None
    };
    let clip = mask_clip(config.mask_mode);
    if clip != MaskClip::Disabled {
        if let Some(mask) = mask.as_ref() {
            let threshold = config.mask_threshold.clamp(1, 255) as u8;
            batch
                .particles
                .retain(|particle| clip.keeps(&mask, particle.x, particle.y, threshold));
            batch
                .trail_images
                .retain(|particle| clip.keeps(&mask, particle.x, particle.y, threshold));
            batch.trail_segments.retain(|segment| {
                clip.keeps(
                    &mask,
                    (segment.from[0] + segment.to[0]) * 0.5,
                    (segment.from[1] + segment.to[1]) * 0.5,
                    threshold,
                )
            });
        }
    }
    let profile_calculation = profile_started.map(|started| started.elapsed());
    let profile_particles = batch.particles.len();
    let mut profile_output_items =
        batch.particles.len() + batch.trail_images.len() + batch.trail_segments.len();
    let mut quads = Vec::with_capacity(batch.trail_segments.len());
    if config.mesh_enabled {
        for segment in build_mesh(
            &batch.particles,
            config.mesh_distance.max(1) as f32,
            config.mesh_max_links.clamp(1, 16) as usize,
            512,
        ) {
            append_line_quad(
                &mut quads,
                segment.from,
                segment.to,
                config.mesh_width.max(1) as f32,
                config.mesh_alpha.clamp(0, 100) as f32 / 100.0,
                [config.mesh_r, config.mesh_g, config.mesh_b]
                    .map(|v| v.clamp(0, 255) as f32 / 255.0),
            );
        }
    }
    for segment in batch.trail_segments {
        append_line_quad(
            &mut quads,
            segment.from,
            segment.to,
            segment.width,
            segment.alpha,
            [1.0, 1.0, 1.0],
        );
    }
    if !quads.is_empty() {
        video.draw_poly(&VertexList::QuadColor(quads), None)?;
    }
    if config.mesh_enabled && config.mesh_faces {
        let rgb =
            [config.mesh_r, config.mesh_g, config.mesh_b].map(|v| v.clamp(0, 255) as f32 / 255.0);
        let alpha = config.mesh_alpha.clamp(0, 100) as f32 / 100.0;
        let triangles: Vec<_> =
            build_mesh_faces(&batch.particles, config.mesh_distance.max(1) as f32, 512)
                .into_iter()
                .map(|face| {
                    face.map(|[x, y, z]| VertexColor {
                        x,
                        y,
                        z,
                        r: rgb[0],
                        g: rgb[1],
                        b: rgb[2],
                        a: alpha,
                    })
                })
                .collect();
        if !triangles.is_empty() {
            video.draw_poly(&VertexList::TriangleColor(triangles), None)?;
        }
    }
    let mut drawer = p4_draw::DrawContext {
        source,
        config: &config,
        object_time: render_time,
        background,
        sequence: std::collections::HashMap::new(),
    };
    drawer.draw_many(video, batch.trail_images)?;
    let mut funnel_config = config.clone();
    if !config.funnel_image_files.trim().is_empty() {
        funnel_config.source_kind = 6;
        funnel_config.image_files = config.funnel_image_files.clone();
        funnel_config.image_random = config.funnel_image_random;
    }
    let mut funnel_drawer = p4_draw::DrawContext {
        source: drawer.source.clone(),
        config: &funnel_config,
        object_time: render_time,
        background: drawer.background.clone(),
        sequence: std::collections::HashMap::new(),
    };
    let ring_count = config.funnel_rings.clamp(1, 8) as usize;
    let mut child_budget = 10_000usize;
    for ring in 0..ring_count {
        let children = build_funnel(
            &batch.particles,
            render_time,
            config.funnel_count.max(0) as usize,
            config.funnel_radius.max(1) as f32 * (ring + 1) as f32 / ring_count as f32,
            config.funnel_scale.clamp(1, 500) as f32 / 100.0,
            config.funnel_angular_speed as f32,
            child_budget,
        );
        child_budget = child_budget.saturating_sub(children.len());
        profile_output_items += children.len();
        let children = children
            .into_iter()
            .map(|mut particle| {
                particle.id = particle.id.saturating_mul(8).saturating_add(ring as u64);
                particle.rz +=
                    config.funnel_self_spin as f32 * (render_time - particle.birth_time) as f32;
                particle
            })
            .collect();
        funnel_drawer.draw_many(video, children)?;
        if child_budget == 0 {
            break;
        }
    }
    drawer.draw_many(video, batch.particles)?;
    video.prevent_post_effect();
    if let (Some(started), Some(calculation)) = (profile_started, profile_calculation) {
        p6_profile::record(
            [video.scene.width, video.scene.height],
            profile_particles,
            profile_output_items,
            calculation,
            started.elapsed(),
        );
    }
    Ok(())
}

fn append_line_quad(
    quads: &mut Vec<[VertexColor; 4]>,
    from: [f32; 3],
    to: [f32; 3],
    width: f32,
    alpha: f32,
    rgb: [f32; 3],
) {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let length = dx.hypot(dy);
    if length <= 1e-6 || !width.is_finite() || !alpha.is_finite() {
        return;
    }
    let nx = -dy / length * width * 0.5;
    let ny = dx / length * width * 0.5;
    let color = |x, y, z| VertexColor {
        x,
        y,
        z,
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
        a: alpha,
    };
    quads.push([
        color(from[0] + nx, from[1] + ny, from[2]),
        color(to[0] + nx, to[1] + ny, to[2]),
        color(to[0] - nx, to[1] - ny, to[2]),
        color(from[0] - nx, from[1] - ny, from[2]),
    ]);
}

#[cfg(feature = "legacy")]
aviutl2::register_filter_plugin!(ParticleFilter);
#[cfg(feature = "stack-basic")]
aviutl2::register_filter_plugin!(StackBasicFilter);
#[cfg(feature = "stack-extension")]
aviutl2::register_filter_plugin!(StackExtensionFilter);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_metadata_builds_on_a_one_megabyte_host_stack() {
        let (count, label, name) = std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(|| {
                let info = ParticleFilter.plugin_info();
                (info.config_items.len(), info.label, info.name)
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(count > 20);
        assert_eq!(label.as_deref(), Some(PARTICLE_LABEL));
        assert!(name.ends_with(PARTICLE_SCRIPT_SUFFIX));
    }

    #[test]
    fn p4_text_file_and_track_controls_decode_from_metadata() {
        let items = build_config_items();
        let config: FilterConfig = items.as_slice().to_struct();
        assert_eq!(config.source_kind, 0);
        assert!(config.sequence_pattern.is_empty());
        assert!(config.audio_file.is_none());
        assert_eq!(config.funnel_rings, 1);
    }

    #[test]
    fn binary_basic_controls_are_checks() {
        let items = build_config_items();
        for name in [
            "画像一覧をランダム",
            "追跡時刻 0現在 1出生",
            "メッシュを使用",
            "面を描画",
            "ファンネル画像をランダム",
            "集結を使用",
            "境界反射を使用",
            "分散を使用",
            "反射で分散",
        ] {
            assert!(items.iter().any(|item| {
                matches!(item, FilterConfigItem::Check(value) if value.name == name)
            }));
            assert!(!items.iter().any(|item| {
                matches!(item, FilterConfigItem::Track(value) if value.name == name)
            }));
        }
    }
}
