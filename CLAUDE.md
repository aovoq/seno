# Seno - マルチAIチャットビューワ

## プロジェクト概要

Claude、ChatGPT、Geminiの3つのAIサービスを同時に表示し、統一入力フィールドから同じテキストを全サービスに送信するmacOSアプリ。

## テクノロジースタック

- **Framework**: Tauri v2 (`unstable` feature必須)
- **Frontend**: Vanilla TypeScript (フレームワークなし)
- **Styling**: CSS Variables + システムダークモード対応
- **Build**: Vite v6
- **Package Manager**: bun

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                Titlebar WebView                         │
├─────────────────┬─────────────────┬─────────────────────┤
│   Claude        │   ChatGPT       │   Gemini            │
│   WebView       │   WebView       │   WebView           │
│                 │                 │                     │
├─────────────────┴─────────────────┴─────────────────────┤
│              Input Bar (main webview)                   │
└─────────────────────────────────────────────────────────┘
```

- 1つのウィンドウに5つのwebviewを配置（Titlebar + 3つのAI + 入力バー）
- `Window::add_child()`で子webviewを動的に追加
- リサイズ時は全webviewの位置・サイズを再計算

## ディレクトリ構造

```
seno/
├── src/                        # フロントエンド（入力バーUI）
│   ├── main.ts                 # 入力バーのロジック、キーボードショートカット (93行)
│   └── styles/main.css         # スタイル定義 (139行)
├── src-tauri/                  # Rustバックエンド
│   ├── src/
│   │   ├── lib.rs              # アプリ初期化、webview構築、メニュー、リサイズ処理 (227行)
│   │   ├── main.rs             # エントリーポイント (7行)
│   │   ├── commands.rs         # Tauriコマンド (127行)
│   │   ├── injector.rs         # 各AIサービス用JS注入スクリプト (157行)
│   │   └── layout.rs           # レイアウト計算エンジン (94行)
│   ├── capabilities/           # 権限設定
│   │   ├── default.json        # デフォルト権限
│   │   └── remote-ai.json      # AIサービスURL許可リスト
│   ├── icons/                  # アプリアイコン
│   ├── Cargo.toml              # Rust依存関係
│   └── tauri.conf.json         # Tauri設定
├── .env.example                # 署名用環境変数テンプレート
├── index.html                  # エントリーHTML
├── vite.config.ts              # Vite設定
├── tsconfig.json               # TypeScript設定
└── package.json                # 依存関係定義
```

## 開発コマンド

```bash
bun install          # 依存関係インストール
bun tauri dev        # 開発モード（ホットリロード有効）
bun tauri build      # リリースビルド（署名なし）
```

## 署名付きリリースビルド

### 前提条件

1. **Apple Developer Program**への登録（年間$99）
2. **Developer ID Application証明書**がキーチェーンにインストール済み

証明書の確認：
```bash
security find-identity -v -p codesigning | grep "Developer ID Application"
```

### 環境変数の設定

`.env.example`をコピーして`.env`を作成し、値を設定：

```bash
# .env
APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
APPLE_ID=your-apple-id@example.com
APPLE_TEAM_ID=YOUR_TEAM_ID
APPLE_PASSWORD=your-app-specific-password
```

- `APPLE_SIGNING_IDENTITY`: 証明書名（括弧を含む値はクォート必須）
- `APPLE_PASSWORD`: App用パスワード（[appleid.apple.com](https://appleid.apple.com)で生成）

### ビルドコマンド

```bash
set -a && source .env && set +a && bun tauri build
```

成功すると以下が出力される：
- `src-tauri/target/release/bundle/macos/Seno.app`（署名・公証済み）
- `src-tauri/target/release/bundle/dmg/Seno_0.1.0_aarch64.dmg`（署名済み）

### 署名の確認

```bash
# 署名情報を表示
codesign -dv --verbose=2 src-tauri/target/release/bundle/macos/Seno.app

# Gatekeeper検証
spctl --assess -vv src-tauri/target/release/bundle/macos/Seno.app
```

正常な署名の場合：
- `Authority=Developer ID Application: ...`
- `Notarization Ticket=stapled`
- `spctl`: `accepted` / `source=Notarized Developer ID`

## 主要なTauriコマンド

| コマンド | 説明 |
|---------|------|
| `send_to_all` | 全AIサービスにテキストを送信 |
| `reload_webview` | 指定webviewをリロード |
| `reload_all` | 全webviewをリロード |
| `new_chat_all` | 全サービスで新規チャットを開始 |
| `update_input_height` | 入力バーの高さ変更時にレイアウト再計算 |
| `zoom_in` / `zoom_out` / `zoom_reset` | AIパネルのズーム制御（50%〜200%） |

## キーボードショートカット

| ショートカット | 動作 |
|---------------|------|
| `Cmd+Enter` | メッセージ送信 |
| `Cmd+Shift+=` / `Cmd+=` | ズームイン |
| `Cmd+-` | ズームアウト |
| `Cmd+0` | ズームリセット（100%） |
| `Cmd+N` | 新規チャット（全サービス） |
| `Cmd+R` | 全サービスをリロード |

## 重要な実装詳細

### マルチWebview (unstable feature)

```rust
// Cargo.toml で features = ["unstable"] が必須
tauri = { version = "2", features = ["unstable"] }
```

- `WebviewBuilder`で各AIサービスのwebviewを構築
- `window.add_child()`で同一ウィンドウ内に複数webviewを配置
- 各webviewはラベル（`claude`, `chatgpt`, `gemini`）で識別

### Google OAuth対応

```rust
// User-AgentをSafariに偽装してWebView検出を回避
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) ...Safari/605.1.15";

// OAuthポップアップを許可
.on_new_window(|_url, _features| NewWindowResponse::Allow)
```

**OAuthフロー**:
1. ユーザーがGeminiパネルでログインをクリック
2. Googleが新しいウィンドウを開く
3. `NewWindowResponse::Allow`でウィンドウ作成を許可
4. ユーザーが新ウィンドウでOAuth認証を完了
5. Cookieが`data_store_identifier`に自動保存
6. 新ウィンドウが閉じ、ログイン状態が維持される

### セッション永続化（macOS専用）

- `data_store_identifier`で固定UUIDを設定
- 各AIサービスごとに異なるUUIDでセッション分離
- UUIDは`lib.rs`の`DATA_STORE_IDS`定数で定義
- WebKitがUUID単位でセッションデータを保存

```rust
DATA_STORE_IDS: [(&str, [u8; 16]); 3] = [
    ("claude", [0xa1, 0xb2, ...]),   // Claude専用UUID
    ("chatgpt", [0xb2, 0xc3, ...]),  // ChatGPT専用UUID
    ("gemini", [0xc3, 0xd4, ...]),   // Gemini専用UUID
];
```

### JS注入（injector.rs）

各AIサービスのUIに合わせたテキスト入力・送信スクリプト：

| サービス | エディタ要素 | 送信ボタン検出 |
|---------|-------------|---------------|
| Claude | `[contenteditable="true"]` (ProseMirror) | `button[aria-label*="Send"]`、日本語: `"送信"`, `"メッセージを送信"` |
| ChatGPT | `#prompt-textarea`（contenteditable divまたはtextarea） | `button[data-testid="send-button"]` |
| Gemini | `.ql-editor[contenteditable="true"]` (Quill)、`rich-textarea` | `button[aria-label*="Send"]`、CSSクラス、material tooltip |

**注入プロセス**:
1. テキストをエスケープ（バックスラッシュ、バッククォート、`$`）
2. エディタ要素を検出
3. テキストを挿入（`execCommand('insertText')`またはvalue設定）
4. `input`イベントをディスパッチ（React/Vueリスナー用）
5. 100ms待機後に送信ボタンをクリック

### リサイズ処理

- Rustの`on_window_event`（`Resized`/`ScaleFactorChanged`）でリサイズを検知
- `scale_factor`を考慮して物理ピクセルを論理ピクセルに変換
- 入力バー高さ更新でウィンドウを取得する際は `get_window("main")` を使う
  - `get_webview_window("main")` は `None` を返すことがある
- 入力バーのレイアウト更新は `main_window.run_on_main_thread` で実行

### 座標系

```
物理ピクセル (PhysicalSize): OSが報告する実際の画面ピクセル（例: 3200x1800）
論理ピクセル (LogicalSize): DPI調整後の座標（例: 1600x900）
Scale Factor: 両者の比率（macOS Retinaディスプレイ = 2.0）

変換式: logical = physical / scale_factor
```

- すべて**論理座標（LogicalPosition/LogicalSize）**を使用
- `scale_factor`を考慮してphysical→logical変換が必要
- 入力バーの高さは`layout.rs`でAtomicU32として管理

### レイアウト計算（layout.rs）

```rust
const INPUT_BAR_MIN: f64 = 89.0;   // 最小高さ
const INPUT_BAR_MAX: f64 = 520.0;  // 最大高さ

struct LayoutMetrics {
    width: f64,
    input_bar_height: f64,
    available_height: f64,  // height - input_bar_height
    panel_width: f64,       // width / 3
    last_panel_width: f64,  // width - (panel_width * 2) 端数調整
}
```

**レイアウト配置**:
- AIパネル: `x = panel_width * index`, `y = 0`, `height = available_height`
- 入力バー: `x = 0`, `y = available_height`, `width = 全幅`

### ズーム機能

- 状態: `ZOOM_LEVEL: AtomicU32`（パーセンテージ、100 = 1.0x）
- 範囲: 50%〜200%
- 増減: ±10%
- 全webviewに同時適用
- アプリ再起動でリセット（永続化なし）

### メニューシステム

| メニュー | 項目 |
|---------|------|
| Seno | About, Services, Hide, Hide Others, Show All, Quit |
| Edit | Undo, Redo, Cut, Copy, Paste, Select All |
| View | Zoom In, Zoom Out, Actual Size |
| Chat | New Chat (All), Reload All |

## データフロー

```
ユーザーがテキストを入力
    ↓
index.html (#unified-input textarea)
    ↓
src/main.ts: inputイベント → resizeTextarea()
    ↓
高さ計算 → invoke("update_input_height", height)
    ↓
Tauri RPC → commands::update_input_height()
    ↓
layout::apply_layout() で4つのwebview位置を再計算
    ↓

ユーザーが「送信」をクリック
    ↓
src/main.ts → invoke("send_to_all", text)
    ↓
commands::send_to_all()
    ↓
各サービスごとに:
  injector::get_send_script() → JavaScript生成
  webview.eval(script) → webview内で実行
    ↓
Claude/ChatGPT/Gemini内で:
  1. エディタ要素を検出
  2. テキストを挿入
  3. inputイベントをディスパッチ
  4. 送信ボタンを検出
  5. 100ms後にclick()
```

## ビルド最適化（Cargo.toml）

```toml
[profile.release]
strip = true        # デバッグシンボル削除（バイナリサイズ削減）
lto = true          # リンク時最適化（コンパイル時間増、実行速度向上）
codegen-units = 1   # 単一コード生成ユニット（最適化向上）
panic = "abort"     # パニック時に即座に終了（バイナリサイズ削減）
```

## コーディング規約

### Rust

- エラー処理：`Result<T, String>`を返し、`.map_err(|e| e.to_string())`で変換
- 状態管理：`std::sync::atomic`を使用（ロックフリー）
- webview操作：`app.get_webview(label)`でOption取得、存在チェック必須

### TypeScript

- DOM要素は`as HTMLElement`でキャスト
- Tauriコマンドは`invoke()`で呼び出し
- textarea高さ: 最小40px、最大340px

### CSS

- CSS Variablesでテーマ対応
- `prefers-color-scheme`でシステム設定に追従
- システムフォント（`-apple-system`）を使用
- ダークモード: 背景 #1a1a1a、アクセント #6366f1
- ライトモード: 背景 #f5f5f5、アクセント #4f46e5

## 注意事項・既知の問題

1. **セレクタの破損リスク**: AIサービスのUI更新によりセレクタが動作しなくなる可能性あり（`injector.rs`を確認）

2. **日本語ラベルのハードコード**: Claudeの送信ボタン検出に`"送信"`、`"メッセージを送信"`が含まれる。他言語では動作しない可能性

3. **unstable API**: Tauri v2のunstable featureは将来変更される可能性あり

4. **macOS専用機能**: `data_store_identifier`はmacOSのみ対応（`#[cfg(target_os = "macos")]`）

5. **ズームが永続化されない**: ズームレベルはメモリ内のみで保持、再起動でリセット

6. **100ms送信遅延**: React/Vueのデバウンス処理に対応するため、テキスト挿入後100ms待機してから送信ボタンをクリック

## トラブルシューティング

### ログインセッションが保持されない
→ `data_store_identifier`が正しく設定されているか確認（macOSのみ有効）

### 特定のAIサービスでテキストが入力されない
→ `injector.rs`のセレクタがUIの変更で無効になっている可能性。各サービスのDOM構造を確認

### ウィンドウリサイズ時にレイアウトが崩れる
→ `scale_factor`の計算を確認、論理座標と物理座標の混在に注意

### 送信ボタンが押されない
→ 送信ボタンのaria-label、data-testid、CSSクラスが変更された可能性

### ビルド時に署名されない（ad-hoc署名になる）
→ 環境変数が正しく読み込まれているか確認。`.env`の値に括弧やスペースが含まれる場合はダブルクォートで囲む

### 公証（Notarization）が失敗する
→ `APPLE_PASSWORD`がApp用パスワードか確認。通常のApple IDパスワードでは動作しない

## 新機能追加時のチェックリスト

- [ ] 新しいTauriコマンドは`commands.rs`に追加し、`lib.rs`の`invoke_handler`に登録
- [ ] AIサービス追加時は`AI_SERVICES`定数と`DATA_STORE_IDS`を更新
- [ ] JSスクリプト追加時は`injector.rs`の`get_send_script`にmatchアーム追加
- [ ] 新しいURLパターンは`capabilities/remote-ai.json`に追加
- [ ] メニュー項目追加時は`lib.rs`のメニュー構築部分を更新

## ファイル概要

| ファイル | 行数 | 役割 |
|---------|------|------|
| `main.rs` | 7 | エントリーポイント、lib.rsへ委譲 |
| `lib.rs` | 227 | アプリ初期化、webview構築、メニュー |
| `commands.rs` | 127 | Tauri IPCコマンド |
| `injector.rs` | 157 | AIサービス用JS注入 |
| `layout.rs` | 94 | レイアウト計算エンジン |
| `main.ts` | 93 | 入力バーロジック |
| `main.css` | 139 | スタイリング（ダーク/ライト） |
