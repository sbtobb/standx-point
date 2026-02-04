# Ratatui Skills

[English](README.md) | [中文](README-zh.md)

> Rust ratatui TUI ライブラリのスキルコレクション

## ディレクトリ構成

```
ratatui-skills/
├── SKILL.md              # メインエントリ（概要 + クイックリファレンス）
├── skills/               # サブモジュールスキル
│   ├── basics/          # ターミナル初期化, アプリ構造, イベントループ
│   ├── layout/          # Constraint, Rect, Flex, エリア分割
│   ├── widgets/         # Block, List, Table, カスタムウィジェット
│   └── styling/         # 色, スタイル, 修飾子, Text/Span/Line
└── references/          # 詳細ドキュメント
    ├── _shared/         # 共有ルール (rust-defaults.md)
    ├── basics/          # アプリ構造, バックエンド
    ├── layout/          # 制約タイプ, Flex モード
    ├── widgets/         # 組み込みウィジェット, カスタムウィジェット
    └── styling/         # 色, テキストスタイリング
```

## 使用方法

### Claude Code スキルとして使用

このディレクトリを Claude Code スキルディレクトリにコピーまたはシンボリックリンク：

```bash
# 方法 1: シンボリックリンク
ln -s /path/to/ratatui-skills ~/.claude/skills/ratatui

# 方法 2: コピー
cp -r /path/to/ratatui-skills ~/.claude/skills/ratatui
```

### トリガーキーワード

以下のキーワードでスキルが有効化：
- `ratatui`, `TUI`, `terminal ui`, `ratatui::run`, `ratatui::init`
- `Layout`, `Constraint`, `Rect`, `Flex`, `horizontal`, `vertical`
- `Block`, `Paragraph`, `List`, `Table`, `Gauge`, `Chart`
- `Style`, `Color`, `Stylize`, `Span`, `Line`, `Text`
- `Widget`, `StatefulWidget`, `ListState`, `TableState`

## モジュール

| モジュール | 用途 | 主要 API |
|------------|------|----------|
| **basics** | ターミナル設定 | `ratatui::run()`, `init()`, `restore()`, `DefaultTerminal` |
| **layout** | 画面レイアウト | `Layout`, `Constraint`, `Rect`, `Flex` |
| **widgets** | UI コンポーネント | `Block`, `List`, `Table`, `Gauge`, `Scrollbar` |
| **styling** | 色とテキスト | `Style`, `Color`, `Stylize`, `Span`, `Line`, `Text` |

## バージョン

- **ratatui**: 0.30.0
- **Rust edition**: 2024
- **最終更新**: 2026-01-19
