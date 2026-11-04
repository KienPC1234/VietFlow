cmake_minimum_required(VERSION 3.20)
project(vietflow_tsf LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

option(VIETFLOW_STATIC_CRT "Use static C runtime library (/MT)" ON)

if(MSVC)
    if(VIETFLOW_STATIC_CRT)
        set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
    else()
        set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL")
    endif()
    add_compile_options(/W4 /EHsc /utf-8)
endif()

# Locate static vietflow_core.lib
if(NOT VIETFLOW_CORE_LIB)
    if(CMAKE_GENERATOR_PLATFORM MATCHES "ARM64|arm64" OR CMAKE_VS_PLATFORM_NAME MATCHES "ARM64|arm64" OR CMAKE_SYSTEM_PROCESSOR MATCHES "ARM64|aarch64|arm64")
        set(VIETFLOW_CORE_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../target/aarch64-pc-windows-msvc/release/vietflow_core.lib")
    elseif(CMAKE_GENERATOR_PLATFORM MATCHES "Win32|x86" OR CMAKE_VS_PLATFORM_NAME MATCHES "Win32|x86" OR CMAKE_SIZEOF_VOID_P EQUAL 4)
        set(VIETFLOW_CORE_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../target/i686-pc-windows-msvc/release/vietflow_core.lib")
    else()
        set(VIETFLOW_CORE_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/vietflow_core.lib")
    endif()
endif()

add_library(vietflow_tsf SHARED
    dllmain.cpp
    class_factory.cpp
    debug_log.cpp
    edit_session.cpp
    ipc.cpp
    register.cpp
    tip.cpp
    vietflow_tsf.def
)

target_include_directories(vietflow_tsf PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../../ffi
)

target_link_libraries(vietflow_tsf PRIVATE
    "${VIETFLOW_CORE_LIB}"
    advapi32
    ole32
    user32
    kernel32
    ws2_32
    userenv
    bcrypt
    ntdll
)

