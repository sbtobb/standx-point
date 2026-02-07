# Ratatui Skills

[English](README.md) | [日本語](README-ja.md)

> Rust ratatui TUI 库技能集合

## 目录结构

```
ratatui-skills/
├── SKILL.md              # 主入口（概览 + 快速参考）
├── skills/               # 子模块技能
│   ├── basics/          # 终端初始化, 应用结构, 事件循环
│   ├── layout/          # 约束, Rect, Flex, 区域分割
│   ├── widgets/         # Block, List, Table, 自定义组件
│   └── styling/         # 颜色, 样式, 修饰符, Text/Span/Line
└── references/          # 详细文档
    ├── _shared/         # 共享规则 (rust-defaults.md)
    ├── basics/          # 应用结构, 后端
    ├── layout/          # 约束类型, Flex 模式
    ├── widgets/         # 内置组件, 自定义组件
    └── styling/         # 颜色, 文本样式
```

## 使用方法

### 作为 Claude Code 技能使用

将此目录复制或符号链接到 Claude Code 技能目录：

```bash
# 方式 1: 符号链接
ln -s /path/to/ratatui-skills ~/.claude/skills/ratatui

# 方式 2: 复制
cp -r /path/to/ratatui-skills ~/.claude/skills/ratatui
```

### 触发关键词

技能在以下关键词时激活：
- `ratatui`, `TUI`, `terminal ui`, `ratatui::run`, `ratatui::init`
- `Layout`, `Constraint`, `Rect`, `Flex`, `horizontal`, `vertical`
- `Block`, `Paragraph`, `List`, `Table`, `Gauge`, `Chart`
- `Style`, `Color`, `Stylize`, `Span`, `Line`, `Text`
- `Widget`, `StatefulWidget`, `ListState`, `TableState`
- 中文关键词：`终端界面`, `布局`, `组件`, `样式` 等

## 模块说明

| 模块 | 用途 | 核心 API |
|------|------|----------|
| **basics** | 终端设置 | `ratatui::run()`, `init()`, `restore()`, `DefaultTerminal` |
| **layout** | 屏幕布局 | `Layout`, `Constraint`, `Rect`, `Flex` |
| **widgets** | UI 组件 | `Block`, `List`, `Table`, `Gauge`, `Scrollbar` |
| **styling** | 颜色与文本 | `Style`, `Color`, `Stylize`, `Span`, `Line`, `Text` |

## 版本信息

- **ratatui**: 0.30.0
- **Rust edition**: 2024
- **最后更新**: 2026-01-19
