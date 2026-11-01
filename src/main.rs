use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use slint::{LogicalPosition, Timer, VecModel};

slint::include_modules!();

/// 模拟菜单数量（以后加功能在这里扩展，并同步修改菜单点击的处理逻辑）
const MENU_COUNT: i32 = 6;
/// 默认菜单编号：悬浮球单击时直接触发的功能（1 ~ MENU_COUNT）
const DEFAULT_MENU: i32 = 1;

fn main() -> Result<(), slint::PlatformError> {
    let ball = FloatingBall::new()?;
    let menu = RadialMenu::new()?;
    let popup = NumberPopup::new()?;

    // ---------- 菜单数据 ----------
    let items: Vec<MenuItem> = (1..=MENU_COUNT)
        .map(|i| MenuItem {
            label: format!("功能 {i}").into(),
        })
        .collect();
    menu.set_items(Rc::new(VecModel::from(items)).into());

    // ---------- 悬浮球事件 ----------
    // 单击：菜单已打开则收起；否则直接执行默认菜单功能
    {
        let menu_w = menu.as_weak();
        let popup_w = popup.as_weak();
        let ball_w = ball.as_weak();
        ball.on_clicked(move || {
            let (Some(menu), Some(popup), Some(ball)) =
                (menu_w.upgrade(), popup_w.upgrade(), ball_w.upgrade())
            else {
                return;
            };
            if menu.window().is_visible() {
                hide_menu(&menu);
            } else {
                show_popup(&popup, &ball, DEFAULT_MENU);
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

    // 拖动：按下时记录窗口位置，拖动中按增量移动窗口
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
                let (ox, oy) = drag_origin.get();
                b.window().set_position(LogicalPosition::new(ox + dx, oy + dy));
            }
            // 拖动时顺便收起菜单
            if let Some(m) = menu_w.upgrade() {
                if m.window().is_visible() {
                    hide_menu(&m);
                }
            }
        });
    }

    // ---------- 环形菜单事件 ----------
    // 点击菜单项：收起菜单，并弹出对应编号的数字窗口
    {
        let menu_w = menu.as_weak();
        let popup_w = popup.as_weak();
        let ball_w = ball.as_weak();
        menu.on_item_clicked(move |index| {
            let (Some(menu), Some(popup), Some(ball)) =
                (menu_w.upgrade(), popup_w.upgrade(), ball_w.upgrade())
            else {
                return;
            };
            hide_menu(&menu);
            show_popup(&popup, &ball, index + 1);
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

    ball.show()?;
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

/// 在悬浮球附近弹出数字窗口（220x150 与 number_popup.slint 中的尺寸一致）
fn show_popup(popup: &NumberPopup, ball: &FloatingBall, number: i32) {
    let sf = ball.window().scale_factor();
    let ball_pos = ball.window().position().to_logical(sf);
    let ball_size = ball.window().size().to_logical(sf);
    popup.window().set_position(LogicalPosition::new(
        ball_pos.x + ball_size.width / 2.0 - 110.0,
        ball_pos.y + ball_size.height / 2.0 - 75.0,
    ));
    popup.set_number(number);
    let _ = popup.window().show();
}
