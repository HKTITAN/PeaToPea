cmake_minimum_required(VERSION 3.22.1)
project(pea_jni)

# Prebuilt pea-core: rust-out/<ANDROID_ABI>/libpea_core.a (see pea-android/README.md)
set(PEA_CORE_DIR "${CMAKE_SOURCE_DIR}/../../../../rust-out/${ANDROID_ABI}" CACHE PATH "dir containing libpea_core.a")
set(PEA_CORE_LIB "${PEA_CORE_DIR}/libpea_core.a")

add_library(pea_jni SHARED pea_jni.c)

if(EXISTS "${PEA_CORE_LIB}")
  target_link_libraries(pea_jni ${PEA_CORE_LIB} log)
else()
  target_sources(pea_jni PRIVATE pea_stub.c)
  target_link_libraries(pea_jni log)
endif()
