# 知识库
## 编译的 exe 运行时一直有命令行窗口在
已解决。在 [main.rs](file:///d:/workspaces/trae/desk_tool/src/main.rs#L3) 顶部加了一行：

```rust
#![windows_subsystem = "windows"]
```

**原因：** Rust 在 Windows 默认编译为控制台（console）子系统程序，启动时会附带一个命令行窗口。加上这个属性后编译为 GUI 子系统，双击 exe 或 `cargo run` 都不会再出现命令行窗口。
**注意：** 加上这行后，终端里将看不到 `println!` 等控制台输出（包括之前那些 ICU4X 的报错日志）。如果调试时需要看输出，临时把这行注释掉即可。