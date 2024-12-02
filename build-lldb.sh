#!/bin/sh

set -eu

. "$(dirname "$0")/build-lldb-info.sh"
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
    BUILD_TARGETS='liblldb debugserver'
    lib_prefix=lib
    lib_suffix=.a
    ;;
  Linux*)
    TARGET_ARCH='X86'
    BUILD_TARGETS='liblldb'
    lib_prefix=lib
    lib_suffix=.a
    ;;
  MINGW* )
    TARGET_ARCH='X86'
    BUILD_TARGETS='liblldb'
    lib_prefix=
    lib_suffix=.lib

    # find msvc path
    PROGRAM_FILES_X86="$(perl -E 'say $ENV{"ProgramFiles(x86)"}')"
    PROGRAM_FILES_X86=${PROGRAM_FILES_X86:-$ProgramFiles}
    PROGRAM_FILES_X86="$(cygpath "$PROGRAM_FILES_X86")"
    VSWHERE="$PROGRAM_FILES_X86/Microsoft Visual Studio/Installer/vswhere.exe"

    VISUAL_STUDI_INSTALL_PATH="$("$VSWHERE" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format text -nologo | grep 'installationPath' | sed 's/[^:]*: //')"
    DEFAULT_MSVC_VERSION="$(cat "$VISUAL_STUDI_INSTALL_PATH/VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt")"
    MSVC_DIR="$VISUAL_STUDI_INSTALL_PATH/VC/Tools/MSVC/$DEFAULT_MSVC_VERSION"
    MSVC_PATH="$MSVC_DIR/bin/HostX64/x64"
    MSVC_LIB="$MSVC_DIR/lib/x64"
    MSVC_INCLUDE="$MSVC_DIR/include"

    # allow specifying with env var
    if [ -z "${WINDOWS_KIT_ROOT:-}" ] || ! [ -d "${WINDOWS_KIT_ROOT:-}" ]; then 
      WINDOWS_KIT_ROOT="$(powershell.exe -C "(Get-ItemProperty -Path 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows Kits\Installed Roots').KitsRoot10")"
    fi 
    if [ -z "${WINDOWS_KIT_ROOT:-}" ] || ! [ -d "${WINDOWS_KIT_ROOT:-}" ]; then 
      WINDOWS_KIT_ROOT="$(powershell.exe -C "(Get-ItemProperty -Path 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots').KitsRoot10")"
    fi 
    WINDOWS_KIT_ROOT="$(cygpath "$WINDOWS_KIT_ROOT")"
    echo "We found windows kit at $WINDOWS_KIT_ROOT" >&2
    for WINDOWS_KIT_VERSION_BIN in "$WINDOWS_KIT_ROOT"/bin/* ; do
        if [ -f "$WINDOWS_KIT_VERSION_BIN"/x64/rc.exe ]; then
          WINDOWS_KIT_VERSION="${WINDOWS_KIT_VERSION_BIN##*/}"
          WINDOWS_KIT_BIN="$WINDOWS_KIT_VERSION_BIN"/x64
          WINDOWS_KIT_UCRT_LIB="$WINDOWS_KIT_ROOT/lib/$WINDOWS_KIT_VERSION/ucrt/x64"
          WINDOWS_KIT_UM_LIB="$WINDOWS_KIT_ROOT/lib/$WINDOWS_KIT_VERSION/um/x64"
          WINDOWS_KIT_UCRT_INCLUDE="$WINDOWS_KIT_ROOT/Include/$WINDOWS_KIT_VERSION/ucrt"
          WINDOWS_KIT_UM_INCLUDE="$WINDOWS_KIT_ROOT/Include/$WINDOWS_KIT_VERSION/um"
          WINDOWS_KIT_SHARED_INCLUDE="$WINDOWS_KIT_ROOT/Include/$WINDOWS_KIT_VERSION/shared"
          echo "using windows kit $WINDOWS_KIT_VERSION" >&2
          break
        fi
    done

    if [ -z "$WINDOWS_KIT_VERSION" ]; then
      echo "No Windows Kit found" >&2
      exit 1
    fi

    export PATH="$PATH:$(cygpath "$MSVC_PATH"):$(cygpath "$WINDOWS_KIT_BIN")"
    export LIB="$(cygpath -wp "$WINDOWS_KIT_UCRT_LIB:$WINDOWS_KIT_UM_LIB:$MSVC_LIB")"
    export INCLUDE="$(cygpath -wp "$WINDOWS_KIT_UCRT_INCLUDE:$WINDOWS_KIT_UM_INCLUDE:$WINDOWS_KIT_SHARED_INCLUDE:$MSVC_INCLUDE")"
    export CC="cl.exe"
    export CXX="cl.exe"

    ;;
  * )
    echo "Unsupported platform" >&2;
    exit 1;
esac

build_config_files="$LLVM_BUILD_DIR/.build-config-version"
current_build_config_version="$(cat "$build_config_files" || :)"

if [ ! -f "$LLVM_BUILD_DIR/build.ninja" ] || [ "$current_build_config_version" != "$BUILD_CONFIG_VERSION" ]; then
  echo "configuration llvm" >&2

  cmake \
    -S "$LLVM_SRC_DIR/llvm" \
    -B "$LLVM_BUILD_DIR" \
    -G Ninja \
    -D CMAKE_OSX_ARCHITECTURES="arm64;x86_64" \
    -D CMAKE_BUILD_TYPE="$CMAKE_BUILD_TYPE" \
    -D CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded \
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

  echo "$BUILD_CONFIG_VERSION" > "$build_config_files"
fi

echo "building llvm" >&2

ninja -C "$LLVM_BUILD_DIR" $BUILD_TARGETS

echo "installing header files" >&2
cmake_install_headers() {
  cmake \
    -DCMAKE_INSTALL_PREFIX="$LLVM_DIR" \
    -DCMAKE_INSTALL_LOCAL_ONLY=YES \
    -DCMAKE_INSTALL_COMPONENT="$1" \
    -P "$2"
}

cmake_install_headers llvm-headers "$LLVM_BUILD_DIR/cmake_install.cmake" > /dev/null
cmake_install_headers lldb-headers "$LLVM_BUILD_DIR/tools/lldb/cmake_install.cmake" > /dev/null

echo "installing library / binary files" >&2

lib_dir="$LLVM_DIR/lib"
mkdir -p "$LLVM_DIR/lib"
find "$LLVM_BUILD_DIR/lib" -type f -name "${lib_prefix}*${lib_suffix}" -exec cp {} "$lib_dir" ';'
mkdir -p "$LLVM_DIR/bin"
cp "$LLVM_BUILD_DIR/bin/debugserver" "$LLVM_DIR/bin" || :
cp "$LLVM_BUILD_DIR/bin/lldb-server" "$LLVM_DIR/bin" || :
