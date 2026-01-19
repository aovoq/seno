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
├── .github/
│   └── workflows/
│       └── publish.yml         # リリース自動化ワークフロー
├── src/                        # フロントエンド（入力バーUI）
│   ├── main.ts                 # 入力バー・アップデートロジック (142行)
│   └── styles/main.css         # スタイル定義 (239行)
├── src-tauri/                  # Rustバックエンド
│   ├── src/
│   │   ├── lib.rs              # アプリ初期化、webview構築、メニュー、リサイズ処理 (327行)
│   │   ├── main.rs             # エントリーポイント (7行)
│   │   ├── commands.rs         # Tauriコマンド (127行)
│   │   ├── injector.rs         # 各AIサービス用JS注入スクリプト (157行)
│   │   └── layout.rs           # レイアウト計算エンジン (94行)
│   ├── capabilities/           # 権限設定
│   │   ├── default.json        # デフォルト権限
│   │   └── remote-ai.json      # AIサービスURL許可リスト
│   ├── icons/                  # アプリアイコン
│   ├── Cargo.toml              # Rust依存関係
│   └── tauri.conf.json         # Tauri設定（バージョン管理）
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

## GitHub Actionsによるリリース

### 概要

`src-tauri/tauri.conf.json`の`version`を更新してmainにpushすると、自動的に：
1. バージョンタグが存在しなければリリースビルドを開始
2. macOS（ARM/Intel）、Windows、Linuxの4並列でビルド
3. macOSは署名・公証（Notarization）を実行
4. GitHub Releasesにドラフトとして作成

### リリース手順

```bash
# 1. tauri.conf.json の version を更新（例: 0.1.1 → 0.1.2）
# 2. コミット & プッシュ
git add .
git commit -m "release: v0.1.2"
git push origin main

# 3. ビルド完了後、ドラフトリリースを公開
gh release edit v0.1.2 --draft=false
```

### GitHub Secrets

以下のSecretsがリポジトリに設定されている必要がある：

| Secret名 | 説明 |
|----------|------|
| `APPLE_CERTIFICATE` | Developer ID証明書(.p12)をbase64エンコード |
| `APPLE_CERTIFICATE_PASSWORD` | .p12のパスワード |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Name (TEAM_ID)` |
| `APPLE_ID` | Apple ID |
| `APPLE_PASSWORD` | App用パスワード |
| `APPLE_TEAM_ID` | チームID |
| `KEYCHAIN_PASSWORD` | CI用キーチェーンパスワード（任意の文字列） |
| `TAURI_SIGNING_PRIVATE_KEY` | 自動アップデート用署名秘密鍵 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 署名鍵のパスワード |

### Secrets設定方法

```bash
# 証明書をbase64化して設定
base64 -i certificate.p12 | gh secret set APPLE_CERTIFICATE

# その他
gh secret set APPLE_CERTIFICATE_PASSWORD --body "パスワード"
gh secret set APPLE_SIGNING_IDENTITY --body "Developer ID Application: Name (TEAM_ID)"
gh secret set APPLE_ID --body "your@email.com"
gh secret set APPLE_PASSWORD --body "app-specific-password"
gh secret set APPLE_TEAM_ID --body "TEAM_ID"
gh secret set KEYCHAIN_PASSWORD --body "任意の文字列"

# 自動アップデート署名鍵（初回のみ）
bunx tauri signer generate -w ~/.tauri/seno.key
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/seno.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body "パスワード"

# 確認
gh secret list
```

### 成果物

| プラットフォーム | ファイル |
|-----------------|---------|
| macOS (Apple Silicon) | `Seno_x.x.x_aarch64.dmg` |
| macOS (Intel) | `Seno_x.x.x_x64.dmg` |
| Windows | `Seno_x.x.x_x64-setup.exe`, `.msi` |
| Linux | `.deb`, `.rpm`, `.AppImage` |
| 自動アップデート用 | `latest.json`, `.tar.gz`, `.sig` |

### ワークフロー監視

```bash
# 実行一覧
gh run list

# 詳細確認
gh run view <run_id>

# 失敗ログ
gh run view <run_id> --log-failed

# リリース一覧
gh release list
```

## ローカル署名付きビルド（オプション）

GitHub Actionsを使わずにローカルでビルドする場合：

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
- `src-tauri/target/release/bundle/dmg/Seno_x.x.x_aarch64.dmg`（署名済み）

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

### 自動アップデート機能

- **プラグイン**: `tauri-plugin-updater`, `tauri-plugin-process`
- **エンドポイント**: GitHub Releases (`latest.json`)
- **署名**: minisign形式（公開鍵は`tauri.conf.json`に埋め込み）

**フロー**:
1. アプリ起動後3秒で`check()`を実行
2. `latest.json`から最新バージョン情報を取得
3. 更新がある場合、タイトルバーに「Update available」ボタンを表示
4. ユーザーがクリックすると`downloadAndInstall()`を実行
5. プログレスバーでダウンロード進捗を表示
6. 完了後`relaunch()`で自動再起動

**設定ファイル**:
```json
// tauri.conf.json
"plugins": {
  "updater": {
    "pubkey": "公開鍵（base64）",
    "endpoints": ["https://github.com/.../latest.json"]
  }
}
```

**権限** (`capabilities/default.json`):
- `updater:default`
- `process:allow-restart`

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

### 自動アップデートが動作しない
→ `tauri.conf.json`の`pubkey`と`endpoints`を確認。GitHub Secretsに`TAURI_SIGNING_PRIVATE_KEY`が設定されているか確認

### アップデート署名エラー
→ 秘密鍵のパスワードが`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`と一致しているか確認

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
| `lib.rs` | 327 | アプリ初期化、webview構築、メニュー、プラグイン登録 |
| `commands.rs` | 127 | Tauri IPCコマンド |
| `injector.rs` | 157 | AIサービス用JS注入 |
| `layout.rs` | 94 | レイアウト計算エンジン |
| `main.ts` | 142 | 入力バー・アップデートチェックロジック |
| `main.css` | 239 | スタイリング（ダーク/ライト、アップデートUI） |
