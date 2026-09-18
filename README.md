# パーティクル(R) AviUtl2 移植

元の「パーティクル(R) ver3.54B」を参考にした、AviUtl2 用の Rust 製フィルタ効果です。元の 32bit Lua DLL は使用しません。元オブジェクトの画像を粒子として複数回描きます。

この 0.2.1 版は、移植計画書の **P2 基本機能と P3 運動・時間・軌跡の一部**を実装したものです。旧版全 32 項目との互換性や、旧版と同じ画像になることは確認できていません。対応状況は [互換性一覧](docs/compatibility.md) を参照してください。

## 使い方

1. [リリース ZIP](release/) にある `.au2pkg.zip` を AviUtl2 のプレビュー画面へドラッグ＆ドロップします。手動配置では `target/release/particle_aviutl2.dll` の拡張子を `.auf2` に変え、AviUtl2 の `Plugin` フォルダへ置きます。
2. AviUtl2 で画像・図形・テキストなどのオブジェクトに「パーティクル(R) 基本版」フィルタを追加します。
3. 出力頻度、方向、寿命などを設定します。出力頻度 100 は毎秒 10 回、出力速度 100 は 1 秒あたり 100 座標単位として扱います。同時発生数の既定値は 1 です。

発生形状は `0=点`、`1=水平線`、`2=箱内`、`3=球面` です。範囲 X/Y/Z は発生領域の大きさで、球面では X を直径として使用します。重力は座標単位/秒²、寿命と開始時間は 1/100 秒単位、透過率と拡大率はパーセント値です。最大描画粒子数は 10,000 個です。

Z 出力方向と Z 拡散角度で奥行き方向への飛行を設定できます。Z 回転の初期角度は粒子ごとに決定し、既定では毎秒 60 度回転します。角度と乱数の消費順は旧 DLL との視覚比較前の暫定仕様です。

P3 グループでは風の開始・終了値と変化時間、個別ばらつき、正弦周期変調、集結、境界反射、分散と急停止、円運動、進行時間、軌跡を設定できます。各機能は既定で無効か強さゼロです。軌跡モードは `0=無効`、`1=残像`、`2=白色帯`、`3=小さな画像点` です。計算順と上限は [P3 仕様メモ](docs/p3-design.md) に記載しています。

## 開発

開発環境は [aviutl2-cli](https://github.com/sevenc-nanashi/aviutl2-cli) の `au2` コマンドです。設定は [aviutl2.toml](aviutl2.toml) にあり、AviUtl2 2.1.9 と `aviutl2` クレート 0.46.1 を指定しています。通常の Windows Rust 環境では以下を実行します。

```powershell
au2 prepare
au2 develop
au2 release
```

Windows x64 と Rust 1.98 以降を想定します。MSVC ターゲットの場合は Visual Studio C++ Build Tools が必要です。計算コアだけを確認する場合は `cargo test -p particle-core` を実行します。この作業環境では MSVC リンカがないため、GNU ターゲット、Rust 同梱 `rust-lld` と MinGW binutils でビルドしました。ソースは MSVC 固有の API を使用していません。

`au2 develop --skip-start` による配置と、AviUtl2 2.1.9 のログで「register filter plugin [パーティクル(R) 基本版]」まで確認しました。実際のエフェクトを操作した画面・出力画像の比較は未実施です。

## 構成

- `crates/particle-core`: ホスト非依存の MT19937、発生・運動・寿命の計算。
- `crates/particle-aviutl2`: 設定 UI、時刻の取得、元画像の描画。
- `docs/compatibility.md`: 旧 32 項目との対応範囲。
- `docs/p3-design.md`: P3 の計算順・時間単位・未確定事項。

元 DLL の解析根拠は [移植計画書](Particle_R_AviUtl2_Port_Plan.md) と `Particle_R_Analysis_Evidence.zip` にあります。乱数生成器とシード混合式は解析値に合わせていますが、粒子ごとの乱数消費順や描画順は旧版から完全復元されていません。
