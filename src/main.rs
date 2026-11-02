// Windows 下编译为 GUI 子系统程序，运行时不附带命令行（控制台）窗口。
// 调试期间如需在终端看 println! 输出，可临时注释掉这一行。
#![windows_subsystem = "windows"]

mod config;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{LogicalPosition, Timer, VecModel};

slint::include_modules!();

/// 模拟菜单数量（以后加功能在这里扩展，并在 trigger 中分发）
const MENU_COUNT: i32 = 6;
/// 默认菜单编号：悬浮球单击时直接触发的功能（1 ~ MENU_COUNT）
const DEFAULT_MENU: i32 = 1;

// 与 note_window.slint 中的窗口尺寸保持一致
const NOTE_W: f32 = 460.0;
const NOTE_H: f32 = 400.0;
// 与 number_popup.slint 中的窗口尺寸保持一致
const POPUP_W: f32 = 220.0;
const POPUP_H: f32 = 150.0;

type SharedConfig = Rc<RefCell<config::Config>>;

fn main() -> Result<(), slint::PlatformError> {
    let ball = FloatingBall::new()?;
    let menu = RadialMenu::new()?;
    let popup = NumberPopup::new()?;
    let note = NoteWindow::new()?;

    // ---------- 配置文件 ----------
    let cfg: SharedConfig = Rc::new(RefCell::new(config::load()));
    note.set_save_dir(cfg.borrow().save_dir.clone().unwrap_or_default().into());

    // ---------- 菜单数据 ----------
    let items: Vec<MenuItem> = (1..=MENU_COUNT)
        .map(|i| MenuItem {
            label: format!("功能 {i}").into(),
        })
        .collect();
    menu.set_items(Rc::new(VecModel::from(items)).into());

    // ---------- 菜单功能分发（index 从 0 开始） ----------
    // 0 = 笔记窗口；其余 = 数字弹窗（占位功能）
    let trigger: Rc<dyn Fn(i32)> = {
        let note_w = note.as_weak();
        let popup_w = popup.as_weak();
        let ball_w = ball.as_weak();
        Rc::new(move |index| {
            let Some(ball) = ball_w.upgrade() else { return };
            if index == 0 {
                if let Some(note) = note_w.upgrade() {
                    show_note(&note, &ball);
                }
            } else if let Some(popup) = popup_w.upgrade() {
                show_popup(&popup, &ball, index + 1);
            }
        })
    };

    // ---------- 悬浮球事件 ----------
    // 单击：菜单已打开则收起；否则执行默认菜单功能
    {
        let menu_w = menu.as_weak();
        let trigger = trigger.clone();
        ball.on_clicked(move || {
            let Some(menu) = menu_w.upgrade() else { return };
            if menu.window().is_visible() {
                hide_menu(&menu);
            } else {
                trigger(DEFAULT_MENU - 1);
            }
        });
    }

    // 双击：展开 / 收起环形菜单
    {
        let menu_w = menu.as_weak();
        let ball_w = ball.as_weak();
        ball.on_double_clicked(move || {
            let (Some(menu), Some(ball)) = (menu_w.upgrade(), ball_w.upgrade()) else {
                return;
            };
            if menu.window().is_visible() {
                hide_menu(&menu);
            } else {
                show_menu(&menu, &ball);
            }
        });
    }

    // 右键：退出程序
    ball.on_right_clicked(|| {
        let _ = slint::quit_event_loop();
    });

    // 拖动：按下时记录窗口位置，拖动中按增量移动窗口，拖动结束写入配置
    let drag_origin: Rc<Cell<(f32, f32)>> = Rc::new(Cell::new((0.0, 0.0)));
    {
        let ball_w = ball.as_weak();
        let drag_origin = drag_origin.clone();
        ball.on_drag_started(move || {
            if let Some(b) = ball_w.upgrade() {
                let sf = b.window().scale_factor();
                let pos = b.window().position().to_logical(sf);
                drag_origin.set((pos.x, pos.y));
            }
        });
    }
    {
        let ball_w = ball.as_weak();
        let menu_w = menu.as_weak();
        ball.on_drag_moved(move |dx, dy| {
            if let Some(b) = ball_w.upgrade() {
                // 增量式移动：dx/dy 是相对“当前”窗口位置测出的偏差，
                // 必须累加到当前位置。如果用“按下点原点 + 位移”，
                // 位移参考系随窗口移动而变化，会产生反馈振荡（越拖越偏、闪烁）
                let (cx, cy) = drag_origin.get();
                let np = (cx + dx, cy + dy);
                drag_origin.set(np);
                b.window().set_position(LogicalPosition::new(np.0, np.1));
            }
            // 拖动时顺便收起菜单
            if let Some(m) = menu_w.upgrade() {
                if m.window().is_visible() {
                    hide_menu(&m);
                }
            }
        });
    }
    {
        let ball_w = ball.as_weak();
        let cfg = cfg.clone();
        ball.on_drag_ended(move || {
            if let Some(b) = ball_w.upgrade() {
                let sf = b.window().scale_factor();
                let pos = b.window().position().to_logical(sf);
                let mut c = cfg.borrow_mut();
                c.ball_x = Some(pos.x);
                c.ball_y = Some(pos.y);
                config::save(&c);
            }
        });
    }

    // ---------- 环形菜单事件 ----------
    // 点击菜单项：收起菜单并执行对应功能
    {
        let menu_w = menu.as_weak();
        let trigger = trigger.clone();
        menu.on_item_clicked(move |index| {
            if let Some(m) = menu_w.upgrade() {
                hide_menu(&m);
            }
            trigger(index);
        });
    }
    // 点击菜单空白区域：收起菜单
    {
        let menu_w = menu.as_weak();
        menu.on_dismiss(move || {
            if let Some(m) = menu_w.upgrade() {
                hide_menu(&m);
            }
        });
    }

    // ---------- 数字弹窗事件 ----------
    {
        let popup_w = popup.as_weak();
        popup.on_close_requested(move || {
            if let Some(p) = popup_w.upgrade() {
                let _ = p.window().hide();
            }
        });
    }

    // ---------- 笔记窗口事件 ----------
    // 保存按钮
    {
        let note_w = note.as_weak();
        let cfg = cfg.clone();
        note.on_save_clicked(move || {
            if let Some(note) = note_w.upgrade() {
                save_note(&note, &cfg);
            }
        });
    }
    // 浏览目录按钮
    {
        let note_w = note.as_weak();
        let cfg = cfg.clone();
        note.on_browse_clicked(move || {
            let Some(path) = rfd::FileDialog::new().pick_folder() else {
                return;
            };
            let dir = path.to_string_lossy().to_string();
            if let Some(note) = note_w.upgrade() {
                note.set_save_dir(dir.clone().into());
                note.set_status("".into());
                note.set_status_icon("".into());
            }
            cfg.borrow_mut().save_dir = Some(dir);
            config::save(&cfg.borrow());
        });
    }
    // 目录输入框回车：立即持久化
    {
        let cfg = cfg.clone();
        note.on_dir_accepted(move |text| {
            let dir = text.trim().to_string();
            if !dir.is_empty() {
                cfg.borrow_mut().save_dir = Some(dir);
                config::save(&cfg.borrow());
            }
        });
    }
    // 关闭按钮
    {
        let note_w = note.as_weak();
        note.on_close_clicked(move || {
            if let Some(n) = note_w.upgrade() {
                let _ = n.window().hide();
            }
        });
    }

    ball.show()?;
    // 把悬浮球窗口从任务栏移除，改为常驻系统托盘
    hide_taskbar_icon_soon(ball.as_weak());
    // 恢复上次拖动到的位置
    {
        let c = cfg.borrow();
        if let (Some(x), Some(y)) = (c.ball_x, c.ball_y) {
            ball.window().set_position(LogicalPosition::new(x, y));
        }
    }

    // ---------- 系统托盘 ----------
    // 托盘保持存活直到事件循环结束；创建失败时退化为无托盘运行
    let tray = setup_tray();
    let _tray_timer = tray.as_ref().map(|t| start_tray_events(ball.as_weak(), t));

    slint::run_event_loop()
}

/// 以悬浮球中心为基准展开环形菜单
fn show_menu(menu: &RadialMenu, ball: &FloatingBall) {
    let sf = ball.window().scale_factor();
    let ball_pos = ball.window().position().to_logical(sf);
    let ball_size = ball.window().size().to_logical(sf);
    // 菜单逻辑尺寸 = (radius + item-size) * 2，与 radial_menu.slint 中的定义保持一致
    let menu_size = (menu.get_radius() + menu.get_item_size()) * 2.0;
    menu.window().set_position(LogicalPosition::new(
        ball_pos.x + ball_size.width / 2.0 - menu_size / 2.0,
        ball_pos.y + ball_size.height / 2.0 - menu_size / 2.0,
    ));
    menu.set_expand(0.0);
    let _ = menu.window().show();
    hide_taskbar_icon_soon(menu.as_weak());
    menu.set_expand(1.0);
}

/// 播放收起动画，结束后隐藏菜单窗口
fn hide_menu(menu: &RadialMenu) {
    menu.set_expand(0.0);
    let weak = menu.as_weak();
    Timer::single_shot(Duration::from_millis(180), move || {
        if let Some(m) = weak.upgrade() {
            // 收起期间用户可能又双击重新展开，只有仍处于收起状态才真正隐藏
            if m.get_expand() < 0.5 {
                let _ = m.window().hide();
            }
        }
    });
}

/// 在悬浮球旁边弹出数字窗口（不遮挡悬浮球）
fn show_popup(popup: &NumberPopup, ball: &FloatingBall, number: i32) {
    popup
        .window()
        .set_position(place_beside_ball(ball, POPUP_W, POPUP_H));
    popup.set_number(number);
    let _ = popup.window().show();
    hide_taskbar_icon_soon(popup.as_weak());
}

/// 在悬浮球旁边打开笔记窗口（不遮挡悬浮球）
fn show_note(note: &NoteWindow, ball: &FloatingBall) {
    note.window()
        .set_position(place_beside_ball(ball, NOTE_W, NOTE_H));
    note.set_status("".into());
    note.set_status_icon("".into());
    let _ = note.window().show();
    hide_taskbar_icon_soon(note.as_weak());
}

/// 重新显示悬浮球（例如通过托盘图标唤起）
fn show_ball(ball: &FloatingBall) {
    let _ = ball.window().show();
    hide_taskbar_icon_soon(ball.as_weak());
}

/// 隐藏窗口在任务栏上的图标。
/// Slint 的 winit 后端是懒创建原生窗口的，show() 之后句柄可能还不存在，
/// 所以这里立即尝试一次，等事件循环跑起来后再补一次。
fn hide_taskbar_icon_soon<T: slint::ComponentHandle + 'static>(weak: slint::Weak<T>) {
    let Some(component) = weak.upgrade() else {
        return;
    };
    hide_taskbar_icon(component.window());
    Timer::single_shot(Duration::from_millis(300), move || {
        if let Some(component) = weak.upgrade() {
            hide_taskbar_icon(component.window());
        }
    });
}

#[cfg(windows)]
fn hide_taskbar_icon(window: &slint::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, COINIT_APARTMENTTHREADED, CLSCTX_ALL,
    };
    use windows::Win32::UI::Shell::{ITaskbarList, TaskbarList};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };

    // 原生窗口还没创建时拿不到句柄，交给调用方的延时重试兜底
    let slint_handle = window.window_handle();
    let Ok(handle) = slint_handle.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(win32.hwnd.get() as *mut core::ffi::c_void);

    unsafe {
        // TOOLWINDOW 扩展样式：任务栏按钮和 Alt+Tab 列表里都不再出现该窗口
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let new_style = (ex_style | WS_EX_TOOLWINDOW.0 as i32) & !(WS_EX_APPWINDOW.0 as i32);
        SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);

        // 已经生成的按钮需要 ITaskbarList 立即移除（与 winit 的 set_skip_taskbar 做法一致）
        static COM_INIT: std::sync::Once = std::sync::Once::new();
        COM_INIT.call_once(|| {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        });
        if let Ok(list) = CoCreateInstance::<_, ITaskbarList>(
            &TaskbarList,
            None::<&windows::core::IUnknown>,
            CLSCTX_ALL,
        ) {
            let _ = list.HrInit();
            let _ = list.DeleteTab(hwnd);
        }
    }
}

#[cfg(not(windows))]
fn hide_taskbar_icon(_window: &slint::Window) {}

struct TrayRuntime {
    // 必须持有 TrayIcon，否则托盘图标会在离开作用域时消失
    _icon: tray_icon::TrayIcon,
    show_id: tray_icon::menu::MenuId,
    quit_id: tray_icon::menu::MenuId,
}

/// 创建托盘图标与右键菜单（“显示悬浮球”“退出”）
fn setup_tray() -> Option<TrayRuntime> {
    use tray_icon::menu::{Menu, MenuItem};

    let show_item = MenuItem::with_id("show", "显示悬浮球", true, None);
    let quit_item = MenuItem::with_id("quit", "退出", true, None);

    let menu = Menu::new();
    menu.append(&show_item).ok()?;
    menu.append(&quit_item).ok()?;

    let icon = tray_icon::Icon::from_rgba(tray_icon_rgba(32), 32, 32).ok()?;
    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip("desk_tool 悬浮球")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .ok()?;

    Some(TrayRuntime {
        _icon: tray,
        show_id: show_item.id().clone(),
        quit_id: quit_item.id().clone(),
    })
}

/// 托盘事件轮询：Windows 上托盘事件走原生消息循环，
/// 用 Slint 的定时器在自己的事件循环里取出并分发。
fn start_tray_events(ball: slint::Weak<FloatingBall>, tray: &TrayRuntime) -> Timer {
    let show_id = tray.show_id.clone();
    let quit_id = tray.quit_id.clone();

    let timer = Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(120),
        move || {
            while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                if event.id == show_id {
                    if let Some(ball) = ball.upgrade() {
                        show_ball(&ball);
                    }
                } else if event.id == quit_id {
                    let _ = slint::quit_event_loop();
                }
            }
            // 左键单击托盘图标也可唤起悬浮球（右键菜单由 tray-icon 自行弹出）
            while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                if let tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Some(ball) = ball.upgrade() {
                        show_ball(&ball);
                    }
                }
            }
        },
    );
    timer
}

/// 代码生成 32×32 的蓝色圆形托盘图标（RGBA，无需额外的图片资源）
fn tray_icon_rgba(size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let radius = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - radius;
            let dy = y as f32 + 0.5 - radius;
            let dist = (dx * dx + dy * dy).sqrt();
            // 边缘 1px 抗锯齿
            let alpha = ((radius - dist).clamp(0.0, 1.0) * 255.0) as u8;
            // 从上到下的蓝色渐变
            let t = y as f32 / size as f32;
            let red = (59.0 + t * 20.0) as u8;
            let green = (130.0 - t * 40.0) as u8;
            let blue = (246.0 - t * 30.0) as u8;
            data.extend_from_slice(&[red, green, blue, alpha]);
        }
    }
    data
}

/// 保存笔记：校验目录 -> 生成时间文件名 -> 写入文件 -> 持久化配置
fn save_note(note: &NoteWindow, cfg: &SharedConfig) {
    let dir = note.get_save_dir().trim().to_string();
    if dir.is_empty() {
        note.set_status_icon("⚠".into());
        note.set_status("请先在上方设置保存目录".into());
        note.set_status_ok(false);
        return;
    }
    let dir_path = std::path::PathBuf::from(&dir);
    if !dir_path.exists() {
        if let Err(e) = std::fs::create_dir_all(&dir_path) {
            note.set_status_icon("".into());
            note.set_status(format!("目录不存在且创建失败：{e}").into());
            note.set_status_ok(false);
            return;
        }
    }
    let filename = format!("{}.md", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    // 写成带 BOM 的 UTF-8：无 BOM 时 TotalCmd F3 等老工具会按系统 ANSI(GBK) 解码导致中文乱码
    let mut data = b"\xEF\xBB\xBF".to_vec();
    data.extend_from_slice(note.get_note_content().as_bytes());
    match std::fs::write(dir_path.join(&filename), &data) {
        Ok(_) => {
            note.set_status_icon("✓".into());
            note.set_status(format!("已保存：{filename}").into());
            note.set_status_ok(true);
            note.set_note_content("".into());
            cfg.borrow_mut().save_dir = Some(dir);
            config::save(&cfg.borrow());
        }
        Err(e) => {
            note.set_status_icon("".into());
            note.set_status(format!("保存失败：{e}").into());
            note.set_status_ok(false);
        }
    }
}

/// 计算窗口位置：放在悬浮球左右两侧中空间较大的一侧，不遮挡悬浮球。
/// win_w / win_h 为窗口逻辑尺寸，返回逻辑坐标。
fn place_beside_ball(ball: &FloatingBall, win_w: f32, win_h: f32) -> LogicalPosition {
    let sf = ball.window().scale_factor();
    let ball_pos = ball.window().position(); // 物理坐标
    let ball_size = ball.window().size(); // 物理尺寸
    let bx = ball_pos.x as f32;
    let by = ball_pos.y as f32;
    let bw = ball_size.width as f32;
    let bh = ball_size.height as f32;
    let ball_cx = bx + bw / 2.0;
    let ball_cy = by + bh / 2.0;

    let (sx, sy, sw, sh) = screen_rect_around(ball_cx, ball_cy);
    let win_wp = win_w * sf;
    let win_hp = win_h * sf;
    let gap = 12.0 * sf;

    let right_space = sx + sw - (bx + bw);
    let left_space = bx - sx;

    let x = if right_space >= left_space {
        // 放右边，放不下时贴屏幕右边缘
        (bx + bw + gap).clamp(sx + gap, (sx + sw - win_wp - gap).max(sx + gap))
    } else {
        // 放左边，放不下时贴屏幕左边缘
        (bx - gap - win_wp).max(sx + gap)
    };
    let y = (ball_cy - win_hp / 2.0).clamp(sy + gap, (sy + sh - win_hp - gap).max(sy + gap));

    LogicalPosition::new(x / sf, y / sf)
}

/// 返回包含指定物理坐标的显示器范围；找不到时回退到主屏，再不行返回 1920x1080。
fn screen_rect_around(px: f32, py: f32) -> (f32, f32, f32, f32) {
    if let Ok(displays) = display_info::DisplayInfo::all() {
        let containing = displays.iter().find(|d| {
            px >= d.x as f32
                && px < (d.x + d.width as i32) as f32
                && py >= d.y as f32
                && py < (d.y + d.height as i32) as f32
        });
        let primary = displays.iter().find(|d| d.is_primary).or(displays.first());
        if let Some(d) = containing.or(primary) {
            return (d.x as f32, d.y as f32, d.width as f32, d.height as f32);
        }
    }
    (0.0, 0.0, 1920.0, 1080.0)
}
