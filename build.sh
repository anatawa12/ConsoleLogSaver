#!/bin/sh

# TODO: parsing args

PROJECT_DIR="$(pwd)"

# input variables
BUILD_RELEASE=false
LLDB_LIB_DIR=${LLDB_LIB_DIR:-"$PROJECT_DIR/llvm/lib"}
LLDB_INCLUDE_DIR=${LLDB_INCLUDE_DIR:-"$PROJECT_DIR/llvm/include"}
LLDB_DEBUGSERVER_PATH=${LLDB_DEBUGSERVER_PATH:-"$PROJECT_DIR/llvm/bin/debugserver"}
CARGO_BUILD_TARGET=$(rustc -vV  | grep '^host: ' | sed 's/^host: //')
ENABLE_GUI=true

while [ "$#" != 0 ]; do
    case "$1" in
      --lldb-lib-dir)
        shift
        if [ "$#" = 0 ]; then
          echo "no arg for --lldb-build-dir" >&2
          exit 1
        fi
        LLDB_LIB_DIR="$(realpath "$1")"
        shift
        ;;
      --lldb-include-dir)
        shift
        if [ "$#" = 0 ]; then
          echo "no arg for --lldb-include-dir" >&2
          exit 1
        fi
        LLDB_INCLUDE_DIR="$(realpath "$1")"
        shift
        ;;
      --debugserver-path)
        shift
        if [ "$#" = 0 ]; then
          echo "no arg for --debugserver-path" >&2
          exit 1
        fi
        LLDB_DEBUGSERVER_PATH="$(realpath "$1")"
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
      --release)
        BUILD_RELEASE=true
        shift
        ;;
      --target)
        shift
        if [ "$#" = 0 ]; then
          echo "no arg for --debugserver-path" >&2
          exit 1
        fi
        CARGO_BUILD_TARGET="$1"
        shift
        ;;
      *)
        echo "unknown option: $1" >&2
        exit 1;
    esac
done

if [ ! -d "$LLDB_LIB_DIR" ]; then
  echo "LLDB_LIB_DIR not found." >&2
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
  CARGO_TARGET_DIR="$PROJECT_DIR/target"
fi

if [ "$BUILD_RELEASE" = "true" ]; then
  CARGO_BUILT_DIR="$CARGO_TARGET_DIR/$CARGO_BUILD_TARGET/release"
else
  CARGO_BUILT_DIR="$CARGO_TARGET_DIR/$CARGO_BUILD_TARGET/debug"
fi

export CARGO_BUILD_TARGET
export CARGO_TARGET_DIR

if [ "$BUILD_RELEASE" = "true" ]; then
  CARGO_PROFILE_ARG="--release"
else
  #CARGO_PROFILE_ARG="--debug"
  CARGO_PROFILE_ARG=
fi

BUILD_TMP_DIR="$CARGO_TARGET_DIR/$CARGO_BUILD_TARGET/cls-temp"
mkdir -p "$BUILD_TMP_DIR"

# general global variable

RUSTFLAGS=
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
    RUSTFLAGS="$RUSTFLAGS -C target-feature=+crt-static"
esac

export RUSTFLAGS

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
CLS_MONO_PATH="$BUILD_TMP_DIR/mono" cargo build $CARGO_PROFILE_ARG -p cls-attach-lib
CLS_ATTACH_LIB_PATH="$CARGO_BUILT_DIR/$CLS_ATTACH_LIB_NAME"

if [ "$OS" = "macos" ]; then
  echo "fixing library load path of $MONO_LIB_NAME"
  BUILT_BINARY_NAME="$(otool -L "$CLS_ATTACH_LIB_PATH" | grep 'libmonobdwgc20stub.dylib' | sed -E -e 's/^\t//g' -e 's/ \(compatibility.*$//')";
  UNITY_MONO_LIB_PATH="@executable_path/../Frameworks/MonoBleedingEdge/MonoEmbedRuntime/osx/libmonobdwgc-2.0.dylib"
  install_name_tool -change "$BUILT_BINARY_NAME" "$UNITY_MONO_LIB_PATH" "$CLS_ATTACH_LIB_PATH"
fi

LLDB_INCLUDE_DIRS="$LLDB_INCLUDE_DIR:${LLDB_INCLUDE_DIRS:-}"
if [ "$OS" = "windows" ]; then
  LLDB_INCLUDE_DIRS="$(cygpath -wp "$LLDB_INCLUDE_DIRS")"
fi
export LLDB_INCLUDE_DIRS

export LLDB_LIB_DIR

if [ "$OS" = "macos" ]; then
  export LLDB_BUNDLE_DEBUGSERVER_PATH="${LLDB_DEBUGSERVER_PATH}"
fi

export LLDB_SYS_CFLAGS='-DLLDB_API='
export CLS_ATTACH_LIB_PATH

echo "building main crate"
# shellcheck disable=SC2086
cargo build $CARGO_PROFILE_ARG -p console-log-saver --features "$CARGO_FEATURES"
