cmake_minimum_required(VERSION 3.21)

project(VietFlowUI VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Qt6 6.5 REQUIRED COMPONENTS Quick QuickControls2 Gui)

qt_standard_project_setup()

set(VIETFLOW_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/..")

# Prebuilt Rust core (C ABI). Build it with:
#   cargo build -p vietflow-core --release
if(WIN32)
    set(VIETFLOW_CORE_LIB
        "${VIETFLOW_ROOT}/target/release/vietflow_core.dll.lib"
        CACHE FILEPATH "Import library of the Rust core")
    set(VIETFLOW_CORE_RUNTIME
        "${VIETFLOW_ROOT}/target/release/vietflow_core.dll"
        CACHE FILEPATH "Rust core DLL")
else()
    find_library(VIETFLOW_CORE_LIB
        NAMES vietflow_core
        PATHS "${VIETFLOW_ROOT}/target/release"
        NO_DEFAULT_PATH)
endif()

if(NOT EXISTS "${VIETFLOW_CORE_LIB}")
    message(FATAL_ERROR
        "Rust core library not found: ${VIETFLOW_CORE_LIB}\n"
        "Run: cargo build -p vietflow-core --release")
endif()

qt_add_executable(vietflow-ui
    WIN32
    src/main.cpp
    src/settingsbackend.h
    src/settingsbackend.cpp
    src/enginebridge.h
    src/enginebridge.cpp
    src/theme.h
)

target_include_directories(vietflow-ui PRIVATE "${VIETFLOW_ROOT}/ffi")
target_link_libraries(vietflow-ui PRIVATE
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::Gui
    "${VIETFLOW_CORE_LIB}"
)

if(MSVC)
    target_compile_options(vietflow-ui PRIVATE /utf-8)
endif()

set(VIETFLOW_QML_FILES
    qml/Main.qml
    qml/components/AppTextArea.qml
    qml/components/AppTextField.qml
    qml/components/Card.qml
    qml/components/GhostButton.qml
    qml/components/HintBanner.qml
    qml/components/IconButton.qml
    qml/components/KeyCap.qml
    qml/components/NavButton.qml
    qml/components/PageView.qml
    qml/components/PrimaryButton.qml
    qml/components/SectionLabel.qml
    qml/components/Select.qml
    qml/components/Sidebar.qml
    qml/components/StatusPill.qml
    qml/components/Toggle.qml

    qml/pages/AboutPage.qml
    qml/pages/AdvancedPage.qml
    qml/pages/ConverterPage.qml
    qml/pages/GamePage.qml
    qml/pages/MacroPage.qml
    qml/pages/OverviewPage.qml
    qml/pages/TypingPage.qml
)

set(VIETFLOW_ASSET_FILES
    assets/app-logo.png
    assets/icons/dashboard-dim.svg
    assets/icons/dashboard-active.svg
    assets/icons/spellcheck-dim.svg
    assets/icons/spellcheck-active.svg
    assets/icons/macro-dim.svg
    assets/icons/macro-active.svg
    assets/icons/game-dim.svg
    assets/icons/game-active.svg
    assets/icons/sparkles-dim.svg
    assets/icons/sparkles-active.svg
    assets/icons/keyboard-dim.svg
    assets/icons/keyboard-active.svg
    assets/icons/settings-dim.svg
    assets/icons/settings-active.svg
)

qt_add_resources(vietflow-ui "vietflow_ui"
    PREFIX "/"
    BASE "${CMAKE_CURRENT_SOURCE_DIR}"
    FILES
        ${VIETFLOW_QML_FILES}
        ${VIETFLOW_ASSET_FILES}
)

if(WIN32 AND EXISTS "${VIETFLOW_CORE_RUNTIME}")
    add_custom_command(TARGET vietflow-ui POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${VIETFLOW_CORE_RUNTIME}"
            "$<TARGET_FILE_DIR:vietflow-ui>/vietflow_core.dll"
        COMMENT "Copying vietflow_core.dll next to vietflow-ui.exe")
endif()

install(TARGETS vietflow-ui
    RUNTIME DESTINATION .)
