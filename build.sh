#!/bin/sh

# TODO: parsing args

PROJECT_DIR="$(pwd)"

# input variables
BUILD_RELEASE=false
LLDB_BUILD_DIR="$PROJECT_DIR/llvm/build"
ENABLE_GUI=true

while [ "$#" != 0 ]; do
    case "$1" in
      -l|--lldb-build-dir)
        shift
        if [ "$#" = 0 ]; then
          echo "not arg platform -l or --lldb-build-dir" >&2
          exit 1
        fi
        LLDB_BUILD_DIR="$(realpath "$1")"
        shift
        ;;
      --enable-gui)
        ENABLE_GUI=true
        shift
        ;;
      --disable-gui)
        ENABLE_GUI=false
        shift
        ;;
      *)
        echo "unknown option: $1" >&2
        exit 1;
    esac
done

if [ -z "$LLDB_BUILD_DIR" ]; then
  echo "-l or --lldb-build-dir not specified" >&2
  exit 1
fi

if [ ! -d "$LLDB_BUILD_DIR" ]; then
  echo "LLDB_BUILD_DIR not found." >&2
  exit 1
fi

set -eu

CARGO_FEATURES=""

if [ "$ENABLE_GUI" = true ]; then
  CARGO_FEATURES="$CARGO_FEATURES,gui"
fi

case $(uname) in
  Darwin*)
    OS=macos
    ;;
  Linux*)
    OS=linux
    ;;
  MINGW* )
    OS=windows
    ;;
  * )
    echo "Unsupported platform" >&2;
    exit 1;
esac

# configuration variables

if [ -z "${CARGO_TARGET_DIR:-}" ]; then
  if [ "$BUILD_RELEASE" = "true" ]; then
    CARGO_BUILT_DIR="$PROJECT_DIR/target/release"
  else
    CARGO_BUILT_DIR="$PROJECT_DIR/target/debug"
  fi
  CARGO_TARGET_DIR="$PROJECT_DIR/target"
fi

export CARGO_TARGET_DIR

if [ "$BUILD_RELEASE" = "true" ]; then
  CARGO_PROFILE_ARG="--release"
else
  #CARGO_PROFILE_ARG="--debug"
  CARGO_PROFILE_ARG=
fi

BUILD_TMP_DIR="$CARGO_TARGET_DIR/cls-temp"
mkdir -p "$BUILD_TMP_DIR"

# general global variable

CARGO_FLAGS=
case ${OS} in
  macos)
    LIB_PREFIX=lib
    DYLIB_SUFFIX=.dylib
    ;;
  linux)
    LIB_PREFIX=lib
    DYLIB_SUFFIX=.so
    ;;
  windows)
    LIB_PREFIX=
    DYLIB_SUFFIX=.dll
    CARGO_FLAGS="$CARGO_FLAGS -C target-feature=+crt-static"
esac

if [ "$OS" = "macos" ] || [ "$OS" = "linux" ]; then
  MONO_STUB_NAME="${LIB_PREFIX}monobdwgc20stub${DYLIB_SUFFIX}"
  MONO_STUB_PATH="$CARGO_BUILT_DIR/$MONO_STUB_NAME"
  MONO_LIB_NAME="${LIB_PREFIX}monobdwgc-2.0${DYLIB_SUFFIX}"
  MONO_LIB_PATH="$BUILD_TMP_DIR/mono/$MONO_LIB_NAME"

  echo "building ${MONO_LIB_NAME} stub file"
  
  cargo build $CARGO_PROFILE_ARG -p monobdwgc20stub
  mkdir -p "$BUILD_TMP_DIR/mono"
  cp "$MONO_STUB_PATH" "$MONO_LIB_PATH"
fi

CLS_ATTACH_LIB_NAME="${LIB_PREFIX}cls_attach_lib${DYLIB_SUFFIX}"
echo "building ${CLS_ATTACH_LIB_NAME}..."
# shellcheck disable=SC2086
CLS_MONO_PATH="$BUILD_TMP_DIR/mono" cargo build $CARGO_PROFILE_ARG $CARGO_FLAGS -p cls-attach-lib
CLS_ATTACH_LIB_PATH="$CARGO_BUILT_DIR/$CLS_ATTACH_LIB_NAME"

if [ "$OS" = "macos" ]; then
  echo "fixing library load path of $MONO_LIB_NAME"
  BUILT_BINARY_NAME="$(otool -L "$CLS_ATTACH_LIB_PATH" | grep 'libmonobdwgc20stub.dylib' | sed -E -e 's/^\t//g' -e 's/ \(compatibility.*$//')";
  UNITY_MONO_LIB_PATH="@executable_path/../Frameworks/MonoBleedingEdge/MonoEmbedRuntime/osx/libmonobdwgc-2.0.dylib"
  install_name_tool -change "$BUILT_BINARY_NAME" "$UNITY_MONO_LIB_PATH" "$CLS_ATTACH_LIB_PATH"
fi

if [ -z "${LLDB_INCLUDE_DIRS:-}" ]; then
  echo "installing header files for llvm"
  LLVM_PREFIX="$BUILD_TMP_DIR/llvm"
  cmake_install_headers() {
    cmake \
      -DCMAKE_INSTALL_PREFIX="$LLVM_PREFIX" \
      -DCMAKE_INSTALL_LOCAL_ONLY=YES \
      -DCMAKE_INSTALL_COMPONENT="$1" \
      -P "$2"
  }

  cmake_install_headers llvm-headers "$LLDB_BUILD_DIR/cmake_install.cmake" > /dev/null
  cmake_install_headers lldb-headers "$LLDB_BUILD_DIR/tools/lldb/cmake_install.cmake" > /dev/null

  LLDB_INCLUDE_DIRS="$LLVM_PREFIX/include"
  export LLDB_INCLUDE_DIRS
fi

if [ -z "${LLDB_LIB_DIR:-}" ]; then
  LLDB_LIB_DIR="$LLDB_BUILD_DIR/lib"
  export LLDB_LIB_DIR
fi

if [ "$OS" = "macos" ] || [ "$OS" = "linux" ]; then
  export LLDB_BUNDLE_DEBUGSERVER_PATH="$LLDB_BUILD_DIR/bin/debugserver"
fi

export LLDB_SYS_CFLAGS='-DLLDB_API='
export CLS_ATTACH_LIB_PATH

echo "building main crate"
# shellcheck disable=SC2086
cargo build $CARGO_PROFILE_ARG $CARGO_FLAGS -p console-log-saver --features ""
