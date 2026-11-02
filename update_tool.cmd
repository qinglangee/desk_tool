@echo off
chcp 65001 >nul
setlocal

:: ========== 在这里改源路径、目标路径 ==========
set "src=d:\workspaces\trae\desk_tool\target\release\desk_tool.exe"
set "dst=d:\software\zhtool\"
:: ==============================================

:: /Y 自动覆盖，不弹出确认提示
copy /Y "%src%" "%dst%"

if %errorlevel% equ 0 (
    echo ✅ 复制成功
) else (
    echo ❌ 复制失败
)

