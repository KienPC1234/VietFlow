cmake_minimum_required(VERSION 3.20)
project(vietflow_ui LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Single source of truth for the version shown by the About page.
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/../VERSION" VIETFLOW_VERSION)
string(STRIP "${VIETFLOW_VERSION}" VIETFLOW_VERSION)

# Default: match the Rust target's CRT (rustc links the dynamic MSVC runtime
# /MD). Use -DVIETFLOW_STATIC_CRT=ON together with
# RUSTFLAGS="-C target-feature=+crt-static" for a fully portable exe that does
# not need the VC++ Redistributable (roughly 0.3-0.4 MB larger).
option(VIETFLOW_STATIC_CRT "Link the MSVC runtime statically (/MT, no VC++ redist)" ON)
if (VIETFLOW_STATIC_CRT)
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
else()
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL")
endif()

set(IMGUI_DIR ${CMAKE_CURRENT_SOURCE_DIR}/third_party/imgui)

add_library(vietflow_ui STATIC
    src/ui_bridge.cpp
    src/app_window.cpp
    src/config.cpp
    src/theme.cpp
    src/ui_manager.cpp
    ${IMGUI_DIR}/imgui.cpp
    ${IMGUI_DIR}/imgui_draw.cpp
    ${IMGUI_DIR}/imgui_tables.cpp
    ${IMGUI_DIR}/imgui_widgets.cpp
    ${IMGUI_DIR}/backends/imgui_impl_win32.cpp
    ${IMGUI_DIR}/backends/imgui_impl_dx11.cpp
)

target_include_directories(vietflow_ui PUBLIC
    src
    ${IMGUI_DIR}
    ${IMGUI_DIR}/backends
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party/nlohmann_json/single_include
)

target_compile_options(vietflow_ui PRIVATE /utf-8 /Gy /Gw /Zc:inline)

target_compile_definitions(vietflow_ui PUBLIC
    _CRT_SECURE_NO_WARNINGS
    NOMINMAX
    WIN32_LEAN_AND_MEAN
    IMGUI_DISABLE_DEBUG_TOOLS
    IMGUI_DISABLE_OBSOLETE_KEYIO
    VIETFLOW_VERSION="${VIETFLOW_VERSION}"
)

