#!/bin/sh

set -eu

LLVM_COMMIT="9152efbb912e341e112f369f5abedfa49e695fac"
LLVM_ARCHIVE_URL="https://github.com/anatawa12/llvm-project/archive/${LLVM_COMMIT}.tar.gz"

PROJECT_DIR="$(pwd)"
LLVM_DIR="${PROJECT_DIR}/llvm"

LLVM_LOCAL_TAR_GZ="$LLVM_DIR/download/$LLVM_COMMIT.tar.gz"
LLVM_SRC_DIR="$LLVM_DIR/src"
LLVM_BUILD_DIR="$LLVM_DIR/build"

mkdir -p "$LLVM_DIR/"
mkdir -p "$LLVM_DIR/download"

if ! [ -f "$LLVM_LOCAL_TAR_GZ" ]; then
  echo "downloading $LLVM_ARCHIVE_URL..."

  curl -L "$LLVM_ARCHIVE_URL" -o "$LLVM_LOCAL_TAR_GZ.part"
  mv "$LLVM_LOCAL_TAR_GZ.part" "$LLVM_LOCAL_TAR_GZ"

  # clear src dir to ensure to extract
  rm -rf "$LLVM_SRC_DIR"
fi

EXTRACT_PROGRESS_FILE="$LLVM_SRC_DIR/.progress"
if [ -f "$EXTRACT_PROGRESS_FILE" ] || [ ! -d "$LLVM_SRC_DIR/cmake" ] || [ ! -d "$LLVM_SRC_DIR/llvm" ] || [ ! -d "$LLVM_SRC_DIR/lldb" ]; then
  echo "extracting ..."
  rm -rf "$LLVM_SRC_DIR"
  mkdir -p "$LLVM_SRC_DIR"

  echo > "$EXTRACT_PROGRESS_FILE"

  TAR_PREFIX="llvm-project-$LLVM_COMMIT"
  tar --strip-components=1 -x -z -f "$LLVM_LOCAL_TAR_GZ" -C "$LLVM_SRC_DIR" "$TAR_PREFIX/cmake/" "$TAR_PREFIX/llvm/" "$TAR_PREFIX/lldb/"

  rm -f "$EXTRACT_PROGRESS_FILE"
fi

#CMAKE_BUILD_TYPE=Debug
CMAKE_BUILD_TYPE=Release

case $(uname) in
  Darwin*)
    TARGET_ARCH='AArch64;X86'
    BUILD_TARGETS='debugserver liblldb'
    ;;
  Linux*)
    TARGET_ARCH='X86'
    BUILD_TARGETS='debugserver liblldb'
    ;;
  MINGW* )
    TARGET_ARCH='X86'
    BUILD_TARGETS='liblldb'
    ;;
  * )
    echo "Unsupported platform" >&2;
    exit 1;
esac

if [ ! -f "$LLVM_BUILD_DIR/build.ninja" ]; then
  cmake \
    -S "$LLVM_SRC_DIR/llvm" \
    -B "$LLVM_BUILD_DIR" \
    -G Ninja \
    -D CMAKE_BUILD_TYPE="$CMAKE_BUILD_TYPE" \
    -D LLVM_ENABLE_PROJECTS=lldb \
    -D LLVM_TARGETS_TO_BUILD="$TARGET_ARCH" \
    -D LLVM_ENABLE_ZLIB=OFF \
    -D LLVM_ENABLE_ZSTD=OFF \
    -D LLVM_INCLUDE_TESTS=OFF \
    -D LLVM_ENABLE_DIA_SDK=OFF \
    -D LLVM_INCLUDE_BENCHMARKS=OFF \
    -D LLDB_ENABLE_LIBEDIT=OFF \
    -D LLDB_ENABLE_CURSES=OFF \
    -D LLDB_ENABLE_LZMA=OFF \
    -D LLDB_ENABLE_LIBXML2=OFF \
    -D LLDB_ENABLE_PYTHON=OFF \
    -D LLDB_ENABLE_LUA=OFF \
    -D LLDB_INCLUDE_TESTS=OFF \

fi

ninja -C "$LLVM_BUILD_DIR" $BUILD_TARGETS
