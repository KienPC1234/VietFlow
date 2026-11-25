cmake_minimum_required(VERSION 3.16)
project(vietflow-fcitx5 VERSION 0.1.5 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Fcitx5Core REQUIRED)
find_package(Fcitx5Config REQUIRED)
find_package(Fcitx5Utils REQUIRED)

set(VIETFLOW_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")
set(VIETFLOW_FFI_DIR "${VIETFLOW_ROOT}/ffi")
set(VIETFLOW_CORE_LIB_DIR "${VIETFLOW_ROOT}/target/release" CACHE PATH "Directory containing libvietflow_core")

find_library(VIETFLOW_CORE_LIB
    NAMES libvietflow_core.a vietflow_core
    PATHS "${VIETFLOW_CORE_LIB_DIR}"
    NO_DEFAULT_PATH
    REQUIRED
)

add_library(vietflow MODULE
    src/engine.cpp
)

set_target_properties(vietflow PROPERTIES
    PREFIX ""
    OUTPUT_NAME "vietflow"
)

target_include_directories(vietflow PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/src"
    "${VIETFLOW_FFI_DIR}"
)

target_link_libraries(vietflow PRIVATE
    Fcitx5::Core
    Fcitx5::Config
    Fcitx5::Utils
    "${VIETFLOW_CORE_LIB}"
    dl
    pthread
    m
)

configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/data/vietflow-addon.conf.in"
    "${CMAKE_CURRENT_BINARY_DIR}/vietflow-addon.conf"
    @ONLY
)

configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/data/vietflow.conf.in"
    "${CMAKE_CURRENT_BINARY_DIR}/vietflow.conf"
    @ONLY
)

install(TARGETS vietflow
    DESTINATION "${FCITX_INSTALL_ADDONDIR}"
)

install(FILES "${CMAKE_CURRENT_BINARY_DIR}/vietflow-addon.conf"
    DESTINATION "${FCITX_INSTALL_PKGDATADIR}/addon"
    RENAME "vietflow.conf"
)

install(FILES "${CMAKE_CURRENT_BINARY_DIR}/vietflow.conf"
    DESTINATION "${FCITX_INSTALL_PKGDATADIR}/inputmethod"
)

foreach(ICON_SIZE 16 24 32 48 128 256)
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/data/icons/${ICON_SIZE}x${ICON_SIZE}/fcitx-vietflow.png"
        DESTINATION "share/icons/hicolor/${ICON_SIZE}x${ICON_SIZE}/apps"
    )
endforeach()

enable_testing()
add_executable(vietflow_fcitx5_test
    tests/fcitx5_engine_test.cpp
    src/engine.cpp
)
target_include_directories(vietflow_fcitx5_test PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/src"
    "${VIETFLOW_FFI_DIR}"
)
target_compile_definitions(vietflow_fcitx5_test PRIVATE
    VIETFLOW_ADDON_SO_PATH="$<TARGET_FILE:vietflow>"
)
target_link_libraries(vietflow_fcitx5_test PRIVATE
    Fcitx5::Core
    Fcitx5::Config
    Fcitx5::Utils
    "${VIETFLOW_CORE_LIB}"
    dl
    pthread
    m
)
add_test(NAME vietflow_fcitx5_test COMMAND vietflow_fcitx5_test)
