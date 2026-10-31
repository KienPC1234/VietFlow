#include "app_window.h"
#include "constants.h"
#include "ui_manager.h"
#include "theme.h"

#include <d3d11.h>
#include <dwmapi.h>
#include <imgui.h>
#include <imgui_impl_win32.h>
#include <imgui_impl_dx11.h>

extern IMGUI_IMPL_API LRESULT ImGui_ImplWin32_WndProcHandler(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);

namespace vietflow {

static ID3D11Device*           g_pd3dDevice = nullptr;
static ID3D11DeviceContext*    g_pd3dDeviceContext = nullptr;
static IDXGISwapChain*         g_pSwapChain = nullptr;
static ID3D11RenderTargetView* g_mainRenderTargetView = nullptr;
static UINT                    g_ResizeWidth = 0, g_ResizeHeight = 0;
static float                   g_dpi_scale = 1.0f;
static bool                    g_ui_ready = false;

static void CreateRenderTarget() {
    ID3D11Texture2D* pBackBuffer = nullptr;
    g_pSwapChain->GetBuffer(0, IID_PPV_ARGS(&pBackBuffer));
    if (pBackBuffer) {
        g_pd3dDevice->CreateRenderTargetView(pBackBuffer, nullptr, &g_mainRenderTargetView);
        pBackBuffer->Release();
    }
}

static void CleanupRenderTarget() {
    if (g_mainRenderTargetView) {
        g_mainRenderTargetView->Release();
        g_mainRenderTargetView = nullptr;
    }
}

static bool CreateDeviceD3D(HWND hWnd) {
    DXGI_SWAP_CHAIN_DESC sd;
    ZeroMemory(&sd, sizeof(sd));
    sd.BufferCount = 2;
    sd.BufferDesc.Width = 0;
    sd.BufferDesc.Height = 0;
    sd.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    sd.BufferDesc.RefreshRate.Numerator = 60;
    sd.BufferDesc.RefreshRate.Denominator = 1;
    sd.Flags = DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH;
    sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    sd.OutputWindow = hWnd;
    sd.SampleDesc.Count = 1;
    sd.SampleDesc.Quality = 0;
    sd.Windowed = TRUE;
    sd.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;

    UINT createDeviceFlags = 0;
    D3D_FEATURE_LEVEL featureLevel;
    const D3D_FEATURE_LEVEL featureLevelArray[2] = { D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0, };
    HRESULT res = D3D11CreateDeviceAndSwapChain(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr, createDeviceFlags,
                                               featureLevelArray, 2, D3D11_SDK_VERSION, &sd,
                                               &g_pSwapChain, &g_pd3dDevice, &featureLevel, &g_pd3dDeviceContext);
    if (res == DXGI_ERROR_UNSUPPORTED) {
        res = D3D11CreateDeviceAndSwapChain(nullptr, D3D_DRIVER_TYPE_WARP, nullptr, createDeviceFlags,
                                           featureLevelArray, 2, D3D11_SDK_VERSION, &sd,
                                           &g_pSwapChain, &g_pd3dDevice, &featureLevel, &g_pd3dDeviceContext);
    }
    if (res != S_OK) return false;

    CreateRenderTarget();
    return true;
}

static void CleanupDeviceD3D() {
    CleanupRenderTarget();
    if (g_pSwapChain) { g_pSwapChain->Release(); g_pSwapChain = nullptr; }
    if (g_pd3dDeviceContext) { g_pd3dDeviceContext->Release(); g_pd3dDeviceContext = nullptr; }
    if (g_pd3dDevice) { g_pd3dDevice->Release(); g_pd3dDevice = nullptr; }
}

// Rebuild the font atlas and re-scale the style when the window moves to a
// monitor with a different DPI (Per-Monitor V2).
static void RebuildUiForDpi(float dpi_scale) {
    if (!g_ui_ready || !ImGui::GetCurrentContext() || !g_pd3dDevice) return;
    if (dpi_scale <= 0.0f) dpi_scale = 1.0f;

    ImGuiIO& io = ImGui::GetIO();
    io.Fonts->Clear();
    SetupFonts(io, dpi_scale);

    ImGui_ImplDX11_InvalidateDeviceObjects();
    ImGui_ImplDX11_CreateDeviceObjects();
    ApplyTheme(dpi_scale);
}

static LRESULT WINAPI WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    if (ImGui_ImplWin32_WndProcHandler(hWnd, msg, wParam, lParam))
        return true;

    switch (msg) {
        case WM_GETMINMAXINFO: {
            // Keep the client area large enough for the tab bar + content so
            // widgets cannot overlap when the user shrinks the window.
            MINMAXINFO* mmi = (MINMAXINFO*)lParam;
            RECT r = { 0, 0, (LONG)(760.0f * g_dpi_scale), (LONG)(560.0f * g_dpi_scale) };
            AdjustWindowRectEx(&r, WS_OVERLAPPEDWINDOW, FALSE, 0);
            mmi->ptMinTrackSize.x = r.right - r.left;
            mmi->ptMinTrackSize.y = r.bottom - r.top;
            return 0;
        }
        case WM_SIZE:
            if (wParam == SIZE_MINIMIZED) return 0;
            g_ResizeWidth = (UINT)LOWORD(lParam);
            g_ResizeHeight = (UINT)HIWORD(lParam);
            return 0;
        case WM_DPICHANGED: {
            RECT* const prcNewWindow = (RECT*)lParam;
            SetWindowPos(hWnd, nullptr,
                prcNewWindow->left, prcNewWindow->top,
                prcNewWindow->right - prcNewWindow->left,
                prcNewWindow->bottom - prcNewWindow->top,
                SWP_NOZORDER | SWP_NOACTIVATE);

            float new_scale = (float)HIWORD(wParam) / 96.0f;
            if (new_scale <= 0.0f) new_scale = 1.0f;
            g_dpi_scale = new_scale;
            RebuildUiForDpi(new_scale);
            return 0;
        }
        case WM_SYSCOMMAND:
            if ((wParam & 0xfff0) == SC_KEYMENU) return 0; // Disable ALT application menu
            break;
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
    }
    return DefWindowProcW(hWnd, msg, wParam, lParam);
}

int RunAppWindow(const std::string& config_path, bool convert_mode, void (*on_saved)()) {
    // Enable Per-Monitor DPI Awareness v2
    HMODULE hUser32 = GetModuleHandleW(L"user32.dll");
    if (hUser32) {
        typedef BOOL(WINAPI* PFN_SetProcessDpiAwarenessContext)(DPI_AWARENESS_CONTEXT);
        auto pfn = (PFN_SetProcessDpiAwarenessContext)GetProcAddress(hUser32, "SetProcessDpiAwarenessContext");
        if (pfn) {
            pfn(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    HINSTANCE hInst = GetModuleHandleW(nullptr);
    int cx_icon = GetSystemMetrics(SM_CXICON);
    int cy_icon = GetSystemMetrics(SM_CYICON);
    int cx_smicon = GetSystemMetrics(SM_CXSMICON);
    int cy_smicon = GetSystemMetrics(SM_CYSMICON);

    HICON hIconBig = (HICON)LoadImageW(hInst, MAKEINTRESOURCEW(1), IMAGE_ICON, cx_icon, cy_icon, LR_DEFAULTCOLOR | LR_SHARED);
    HICON hIconSmall = (HICON)LoadImageW(hInst, MAKEINTRESOURCEW(1), IMAGE_ICON, cx_smicon, cy_smicon, LR_DEFAULTCOLOR | LR_SHARED);
    if (!hIconBig) {
        hIconBig = LoadIconW(nullptr, (LPCWSTR)IDI_APPLICATION);
        hIconSmall = hIconBig;
    }

    WNDCLASSEXW wc = {
        sizeof(wc), CS_CLASSDC, WndProc, 0L, 0L,
        hInst, hIconBig, nullptr, nullptr, nullptr,
        WINDOW_CLASS_UI, hIconSmall
    };
    RegisterClassExW(&wc);

    UINT dpi = 96;
    if (hUser32) {
        typedef UINT(WINAPI* PFN_GetDpiForSystem)();
        auto pfn = (PFN_GetDpiForSystem)GetProcAddress(hUser32, "GetDpiForSystem");
        if (pfn) {
            dpi = pfn();
        }
    }
    float dpi_scale = (float)dpi / 96.0f;
    if (dpi_scale <= 0.0f) dpi_scale = 1.0f;
    g_dpi_scale = dpi_scale;

    const int init_w = (int)(980.0f * dpi_scale);
    const int init_h = (int)(660.0f * dpi_scale);

    // Center inside the primary monitor's work area (excludes taskbar) instead
    // of the full virtual screen, which is wrong on multi-monitor setups.
    POINT origin = { 0, 0 };
    MONITORINFO mi = { sizeof(mi) };
    int pos_x = 0;
    int pos_y = 0;
    if (GetMonitorInfoW(MonitorFromPoint(origin, MONITOR_DEFAULTTOPRIMARY), &mi)) {
        const int work_w = mi.rcWork.right - mi.rcWork.left;
        const int work_h = mi.rcWork.bottom - mi.rcWork.top;
        pos_x = mi.rcWork.left + (work_w - init_w) / 2;
        pos_y = mi.rcWork.top + (work_h - init_h) / 2;
    } else {
        pos_x = (GetSystemMetrics(SM_CXSCREEN) - init_w) / 2;
        pos_y = (GetSystemMetrics(SM_CYSCREEN) - init_h) / 2;
    }

    const wchar_t* window_title = convert_mode
        ? WINDOW_TITLE_CONVERT
        : WINDOW_TITLE_SETTINGS;

    HWND hwnd = CreateWindowW(wc.lpszClassName, window_title,
                              WS_OVERLAPPEDWINDOW, pos_x, pos_y, init_w, init_h,
                              nullptr, nullptr, wc.hInstance, nullptr);

    if (!hwnd) {
        UnregisterClassW(wc.lpszClassName, wc.hInstance);
        return 1;
    }

    // Explicitly set Big & Small icons on window
    SendMessageW(hwnd, WM_SETICON, ICON_BIG, (LPARAM)hIconBig);
    SendMessageW(hwnd, WM_SETICON, ICON_SMALL, (LPARAM)hIconSmall);

    // Query exact DPI for window if available
    if (hUser32) {
        typedef UINT(WINAPI* PFN_GetDpiForWindow)(HWND);
        auto pfn = (PFN_GetDpiForWindow)GetProcAddress(hUser32, "GetDpiForWindow");
        if (pfn) {
            UINT win_dpi = pfn(hwnd);
            if (win_dpi > 0) {
                dpi = win_dpi;
                dpi_scale = (float)dpi / 96.0f;
                g_dpi_scale = dpi_scale;
            }
        }
    }

    // Windows 11 Dark Mode Title Bar
    BOOL dark_mode = TRUE;
    DwmSetWindowAttribute(hwnd, 20 /* DWMWA_USE_IMMERSIVE_DARK_MODE */, &dark_mode, sizeof(dark_mode));

    if (!CreateDeviceD3D(hwnd)) {
        CleanupDeviceD3D();
        DestroyWindow(hwnd);
        UnregisterClassW(wc.lpszClassName, wc.hInstance);
        return 1;
    }

    ShowWindow(hwnd, SW_SHOWDEFAULT);
    UpdateWindow(hwnd);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    // Do not drop imgui.ini next to the executable (may be read-only, e.g.
    // Program Files) and do not persist window/table state between launches.
    io.IniFilename = nullptr;

    SetupFonts(io, dpi_scale);
    ApplyTheme(dpi_scale);

    ImGui_ImplWin32_Init(hwnd);
    ImGui_ImplDX11_Init(g_pd3dDevice, g_pd3dDeviceContext);
    g_ui_ready = true;

    UiManager ui_mgr(config_path, convert_mode, (void*)hwnd);
    if (on_saved) {
        ui_mgr.SetOnSavedCallback(on_saved);
    }

    bool done = false;
    bool device_released = false;
    while (!done) {
        MSG msg;
        while (PeekMessageW(&msg, nullptr, 0U, 0U, PM_REMOVE)) {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
            if (msg.message == WM_QUIT) {
                OutputDebugStringA("[VietFlow ImGui] WM_QUIT received, exiting loop\n");
                done = true;
            }
        }
        if (done) break;

        if (g_ResizeWidth != 0 && g_ResizeHeight != 0 && !device_released) {
            CleanupRenderTarget();
            g_pSwapChain->ResizeBuffers(0, g_ResizeWidth, g_ResizeHeight, DXGI_FORMAT_UNKNOWN, 0);
            g_ResizeWidth = g_ResizeHeight = 0;
            CreateRenderTarget();
        }

        // While minimized there is no visible swap chain. Release the D3D11
        // device, swap chain and render target instead of just skipping frames;
        // recreate them when the window is restored.
        if (IsIconic(hwnd)) {
            if (!device_released) {
                g_ui_ready = false;
                ImGui_ImplDX11_Shutdown();
                CleanupDeviceD3D();
                device_released = true;
            }
            Sleep(16);
            continue;
        }

        if (device_released) {
            if (!CreateDeviceD3D(hwnd)) {
                CleanupDeviceD3D();
                done = true;
                break;
            }
            g_ResizeWidth = g_ResizeHeight = 0;
            ImGui_ImplDX11_Init(g_pd3dDevice, g_pd3dDeviceContext);
            g_ui_ready = true;
            RebuildUiForDpi(g_dpi_scale);
            device_released = false;
        }

        ImGui_ImplDX11_NewFrame();
        ImGui_ImplWin32_NewFrame();
        ImGui::NewFrame();

        ui_mgr.Render();

        ImGui::Render();
        const float clear_color[4] = { 0.06f, 0.07f, 0.09f, 1.00f };
        g_pd3dDeviceContext->OMSetRenderTargets(1, &g_mainRenderTargetView, nullptr);
        g_pd3dDeviceContext->ClearRenderTargetView(g_mainRenderTargetView, clear_color);
        ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());

        g_pSwapChain->Present(1, 0); // V-Sync enabled for 0% CPU idle
    }

    g_ui_ready = false;
    if (!device_released) {
        ImGui_ImplDX11_Shutdown();
    }
    ImGui_ImplWin32_Shutdown();
    ImGui::DestroyContext();

    if (!device_released) {
        CleanupDeviceD3D();
    }
    DestroyWindow(hwnd);
    UnregisterClassW(wc.lpszClassName, wc.hInstance);

    return 0;
}

} // namespace vietflow
