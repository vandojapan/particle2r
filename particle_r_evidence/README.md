# パーティクル(R) ver3.54B 静的解析の証拠

調査日：2026-09-18。添付ZIPを静的に解析した結果。DLL/EXEの実行、ホスト実機検証、移植プラグインのビルドは行っていない。

## 内容

- `binary_summary.json`：PE構造、指紋、ファイル構成、Lua登録関数。
- `inventory.json`：元ZIP内185ファイルの名前・サイズ・SHA-256。
- `lua_exports.json`：Luaに登録される6関数の開始RVA/VA。PE公開関数は別に `luaopen_particle_set3` の1個。
- `pe_headers.txt`：objdump -x出力。インポート・エクスポート・セクション・再配置。
- `annotated_disassembly.txt`：objdump -d -Mintel出力へ.rdata文字列候補とLua import trampoline名を付けたもの。
- `string_xrefs.json`：.rdata文字列候補の即値参照。データが文字列として誤検出される場合があり、参照の存在だけで処理の意味は確定しない。
- `parameters.json`：32効果のUI宣言、元行番号、設定配列、DLL呼出。
- `Parameter_Catalog.md`：同上の閲覧用カタログ。Luaダイアログの構文は原文として保持。
- `key_disassembly.txt`：登録処理、MT乱数、基本値変換、シード混合、ホスト関数保存の主要部分。
- `upstream_sources.json`：参照したaviutl2-rsソースのURLと取得内容のハッシュ。
- `reproduce_static.py`：元ZIPから証拠を再生成するPythonスクリプト。
- `checks.json`：抽出結果の整合検査。DLL挙動のテスト結果ではない。

元DLL、EXE、画像、AUP等そのものは同梱していない。逆アセンブルはユーザー提供DLLから作成した解析資料。

## 再現

Python 3とGNU objdumpが必要。Linux/WSLで以下を実行する。

```bash
python reproduce_static.py 'パーティクル(R).zip' output
```

DLLのSHA-256を確認し、対象版以外なら停止する。Lua登録テーブルのRVAとimport trampolineの並びはこのDLL用に確認した値であり、別版へ自動適用しない。

## 数値処理の復元メモ

- RNG初期化RVA 0xACB0、配列初期化0xACF0、次乱数0xAE10。
- MTの状態は624要素。temperingは `y ^= y>>11; y ^= (y<<7)&0x9D2C5680; y ^= (y<<15)&0xEFC60000; y ^= y>>18`。
- メインのシード混合式は移植計画書4.3節に記載。実数化・再シード箇所・乱数消費順の完全復元は未完了。
- 基本出力頻度から10/frequencyを計算。
- 描写精度は整数化し最低1。フレームレートも一部の時間計算で整数化。
- `ac` に0.01を掛ける処理があるが、その値を最終物理単位と即断しない。
- スクリプト内のグローバル値、ホスト描画関数、各カーブはコアの明示入力へ変換する。

この資料は全関数のデコンパイル済みソースではない。逆アセンブル全体には標準ランタイムやデータの誤逆アセンブルも含まれる。アドレスをRVAへ変換するときはImageBase 0x6B700000を差し引く。
