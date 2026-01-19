# Seno - マルチAIチャットビューワ

## プロジェクト概要

Claude、ChatGPT、Geminiの3つのAIサービスを同時に表示し、統一入力フィールドから同じテキストを全サービスに送信するmacOSアプリ。

## テクノロジースタック

- **Framework**: Tauri v2 (`unstable` feature必須)
- **Frontend**: Vanilla TypeScript (フレームワークなし)
- **Styling**: CSS Variables + システムダークモード対応
- **Build**: Vite
- **Package Manager**: npm

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                    Main Window                          │
├─────────────────┬─────────────────┬─────────────────────┤
│   Claude        │   ChatGPT       │   Gemini            │
│   WebView       │   WebView       │   WebView           │
│                 │                 │                     │
├─────────────────┴─────────────────┴─────────────────────┤
│              Input Bar (main webview)                   │
└─────────────────────────────────────────────────────────┘
```

- 1つのウィンドウに4つのwebviewを配置（3つのAI + 入力バー）
- `Window::add_child()`で子webviewを動的に追加
- リサイズ時は全webviewの位置・サイズを再計算

## ディレクトリ構造

```
seno/
├── src/                        # フロントエンド（入力バーUI）
│   ├── main.ts                 # 入力バーのロジック、キーボードショートカット
│   └── styles/main.css         # スタイル定義
├── src-tauri/                  # Rustバックエンド
│   ├── src/
│   │   ├── lib.rs              # アプリ初期化、webview構築、リサイズ処理
│   │   ├── main.rs             # エントリーポイント
│   │   ├── commands.rs         # Tauriコマンド（send_to_all, zoom等）
│   │   ├── injector.rs         # 各AIサービス用JS注入スクリプト
│   │   └── layout.rs           # 入力バー高さの状態管理
│   ├── capabilities/           # 権限設定（remote-ai.json等）
│   └── tauri.conf.json         # Tauri設定
├── index.html                  # エントリーHTML
├── vite.config.ts              # Vite設定
└── package.json
```

## 開発コマンド

```bash
npm install          # 依存関係インストール
npm run tauri dev    # 開発モード（ホットリロード有効）
npm run tauri build  # リリースビルド
```

## 主要なTauriコマンド

| コマンド | 説明 |
|---------|------|
| `send_to_all` | 全AIサービスにテキストを送信 |
| `reload_webview` | 指定webviewをリロード |
| `update_input_height` | 入力バーの高さ変更時にレイアウト再計算 |
| `zoom_in/zoom_out/zoom_reset` | AIパネルのズーム制御 |

## キーボードショートカット

| ショートカット | 動作 |
|---------------|------|
| `Cmd+Enter` | メッセージ送信 |
| `Cmd++` / `Cmd+=` | ズームイン |
| `Cmd+-` | ズームアウト |
| `Cmd+0` | ズームリセット |

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

### セッション永続化

- `data_store_identifier`で固定UUIDを設定（macOS専用）
- 各AIサービスごとに異なるUUIDでセッション分離
- UUIDは`lib.rs`の`DATA_STORE_IDS`定数で定義

### JS注入（injector.rs）

各AIサービスのUIに合わせたテキスト入力・送信スクリプト：

| サービス | エディタ要素 | 送信ボタン検出 |
|---------|-------------|---------------|
| Claude | `[contenteditable="true"]` (ProseMirror) | `button[aria-label*="Send"]` |
| ChatGPT | `#prompt-textarea` | `button[data-testid="send-button"]` |
| Gemini | `.ql-editor[contenteditable="true"]` | `button[aria-label*="Send"]` |

### リサイズ処理

- Rustの`on_window_event`（`Resized`/`ScaleFactorChanged`）でリサイズを検知し、レイアウトを再計算
- `scale_factor`を考慮して物理ピクセルを論理ピクセルに変換
- 入力バー高さ更新でウィンドウを取得する際は `get_window("main")` を使う（`get_webview_window("main")` は `None` になりやすい）
- 入力バーのレイアウト更新は `main_window.run_on_main_thread` で実行する

### 座標系

- すべて**論理座標（LogicalPosition/LogicalSize）**を使用
- `scale_factor`を考慮してphysical→logical変換が必要
- 入力バーの高さは`layout.rs`でAtomicU32として管理（76px〜300px）

## コーディング規約

### Rust

- エラー処理：`Result<T, String>`を返し、`.map_err(|e| e.to_string())`で変換
- 状態管理：`std::sync::atomic`を使用（ロックフリー）
- webview操作：`app.get_webview(label)`でOption取得、存在チェック必須

### TypeScript

- DOM要素は`as HTMLElement`でキャスト
- Tauriコマンドは`invoke()`で呼び出し

### CSS

- CSS Variablesでテーマ対応
- `prefers-color-scheme`でシステム設定に追従
- システムフォント（`-apple-system`）を使用

## 注意事項・既知の問題

1. **セレクタの破損リスク**: AIサービスのUI更新によりセレクタが動作しなくなる可能性あり（`injector.rs`を確認）

2. **unstable API**: Tauri v2のunstable featureは将来変更される可能性あり

3. **macOS専用機能**: `data_store_identifier`はmacOSのみ対応（`#[cfg(target_os = "macos")]`）

4. **ズーム範囲**: 50%〜200%（`commands.rs`で定義）

5. **入力バー高さ**: 76px〜300px（`layout.rs`で定義）

## トラブルシューティング

### ログインセッションが保持されない
→ `data_store_identifier`が正しく設定されているか確認

### 特定のAIサービスでテキストが入力されない
→ `injector.rs`のセレクタがUIの変更で無効になっている可能性

### ウィンドウリサイズ時にレイアウトが崩れる
→ `scale_factor`の計算を確認、論理座標と物理座標の混在に注意

## 新機能追加時のチェックリスト

- [ ] 新しいTauriコマンドは`commands.rs`に追加し、`lib.rs`の`invoke_handler`に登録
- [ ] AIサービス追加時は`AI_SERVICES`定数と`DATA_STORE_IDS`を更新
- [ ] JSスクリプト追加時は`injector.rs`の`get_send_script`にmatchアーム追加
