# desk_tool

悬浮于桌面的圆形快捷工具：单击悬浮球直接执行默认功能，双击展开环形菜单，菜单项以悬浮球为中心呈环形展开。

## 环境要求

- Windows（当前主要验证平台）
- Rust 工具链 1.85+（安装：<https://rustup.rs/>）

## 编译

```powershell
cargo build
```

首次编译需要下载并构建 Slint 及其依赖，耗时几分钟。编译产物在 `target\debug\desk_tool.exe`。

发布版本（体积更小、运行更快）：

```powershell
cargo build --release
```

## 运行

```powershell
cargo run
```

或直接双击编译产物 `target\debug\desk_tool.exe`。

## 交互说明

| 操作 | 效果 |
|---|---|
| 左键拖动悬浮球 | 移动悬浮球（拖动超过 3px 判定为拖拽，不触发点击） |
| 单击悬浮球 | 直接执行默认菜单功能（当前为"菜单 1"） |
| 双击悬浮球 | 展开 / 收起环形菜单 |
| 点击菜单项 | 执行对应功能（当前为弹出编号窗口），菜单自动收起 |
| 点击菜单空白区域 | 收起菜单 |
| 点击弹窗任意处 | 关闭弹窗 |
| 右键点击悬浮球 | 退出程序 |

## 常见问题

### 窗口背景不透明（出现黑色或白色矩形底）

默认渲染器在你的机器上不支持透明窗口，切换渲染器：

```powershell
$env:SLINT_BACKEND="winit-skia"; cargo run
```

### 编译报错

确认 Rust 版本（`rustc --version`）不低于 1.85；仍报错请附上完整错误信息反馈。

## 项目结构

```
desk_tool/
├── Cargo.toml            # 依赖：slint / slint-build
├── build.rs              # 编译 ui/main.slint
├── src/
│   └── main.rs           # 窗口逻辑：菜单位置、事件分发、拖拽、展开/收起
└── ui/
    ├── main.slint        # 组件导出入口
    ├── floating_ball.slint   # 通用悬浮球控件（外观可配置，内置单击/双击区分）
    ├── radial_menu.slint     # 环形菜单（数据驱动，回弹展开动画）
    └── number_popup.slint    # 数字弹窗
```

## 如何扩展菜单功能

1. 修改 `src/main.rs` 顶部的 `MENU_COUNT` 和 items 列表添加菜单项；
2. 在 `menu.on_item_clicked` 回调中按 `index` 分发到真实功能；
3. 修改 `DEFAULT_MENU` 常量可更换悬浮球单击触发的默认功能。
