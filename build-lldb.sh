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

    # utilities
    get_registry() {
      until [ -n "${_registry:-}" ]
      do
        _registry=$(powershell.exe -C "if (Test-Path 'Registry::$1') { (Get-ItemProperty -Path 'Registry::$1').$2 }")
        shift 2
      done
      echo "$_registry"
    }

    find_version() {
      # WINDOWS_KIT_UCRT_LIB=$(find_version "$WINDOWS_KIT_ROOT" 'lib' 'ucrt')

      # $4 is prev pwd, $5 is result
      set -- "$1" "$2" "$3" "$(pwd)" ""

      cd "$WINDOWS_KIT_ROOT/$1"
      for KIT_VERSION in * ; do
        # skip not starting with 10
        case "$KIT_VERSION" in
          10.* ) ;;
          *) continue ;;
        esac

        if [ -e "$WINDOWS_KIT_ROOT/$1/$KIT_VERSION/$2" ]; then
          set -- "$1" "$2" "$3" "$4" "$KIT_VERSION"
        fi
      done

      cd "$4"
      echo "$5"
    }

    escape_sh() {
      # escape_sh "your string needs ' escape"
      # => will get 'your string needs '\'' escape'
      # this is for eval

      printf "'%s'\n" "$(printf "%s" "$1" | sed "s/'/'\\\\''/")" 
    }

    append_path() {
      # append_path PATH /your/path
      # will be PATH="$PATH:/your/path" or PATH="$/your/path"
      set "$1" "$2" "$(eval "echo \${$1:-}")"
      if [ -z "$3" ]; then
        eval "$1=$(escape_sh "$2")"
      else
        eval "$1=$(escape_sh "$3"):$(escape_sh "$2")"
      fi
    }

    # find msvc path
    PROGRAM_FILES_X86="$(perl -E 'say $ENV{"ProgramFiles(x86)"}')"
    ANY_PROGRAM_FILES=${PROGRAM_FILES_X86:-$ProgramFiles}
    ANY_PROGRAM_FILES="$(cygpath "$ANY_PROGRAM_FILES")"
    VSWHERE="$ANY_PROGRAM_FILES/Microsoft Visual Studio/Installer/vswhere.exe"

    VISUAL_STUDIO_INSTALL_PATH="$("$VSWHERE" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format text -nologo | grep 'installationPath' | sed 's/[^:]*: //')"
    VISUAL_STUDIO_INSTALL_PATH="$(cygpath "$VISUAL_STUDIO_INSTALL_PATH")"
    DEFAULT_MSVC_VERSION="$(cat "$VISUAL_STUDIO_INSTALL_PATH/VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt")"
    echo "Using Visual Studio $VISUAL_STUDIO_INSTALL_PATH" >&2
    echo "Using MSVC $DEFAULT_MSVC_VERSION" >&2

    MSVC_DIR="$VISUAL_STUDIO_INSTALL_PATH/VC/Tools/MSVC/$DEFAULT_MSVC_VERSION"

    append_path PATH "$MSVC_DIR/bin/HostX64/x64"
    append_path LIB_CYGPATH "$MSVC_DIR/lib/x64"
    append_path INCLUDE_CYGPATH "$MSVC_DIR/include"

    if [ -z "${WINDOWS_KIT_ROOT:-}" ] || ! [ -d "${WINDOWS_KIT_ROOT:-}" ]; then 
      WINDOWS_KIT_ROOT="$(get_registry \
        "HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows Kits\Installed Roots" 'KitsRoot10' \
        "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots" 'KitsRoot10'\
      )"
    fi
    WINDOWS_KIT_ROOT="$(cygpath "$WINDOWS_KIT_ROOT")"
    echo "We found windows kit at $WINDOWS_KIT_ROOT" >&2

    if [ -z "${WINDOWS_SDK_ROOT:-}" ] || ! [ -d "${WINDOWS_SDK_ROOT:-}" ]; then 
      WINDOWS_SDK_ROOT="$(get_registry \
        "HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Microsoft SDKs\Windows\v10.0" 'InstallationFolder' \
        "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0" 'InstallationFolder'\
      )"
    fi
    WINDOWS_SDK_ROOT="$(cygpath "$WINDOWS_SDK_ROOT")"
    echo "We found windows 10 SDK at $WINDOWS_SDK_ROOT" >&2

    WINDOWS_KIT_VERSION="$(find_version "$WINDOWS_KIT_ROOT" lib ucrt)"
    
    if [ -z "$WINDOWS_KIT_VERSION" ]; then
      echo "No Windows Kit Version found" >&2
      exit 1
    fi

    WINDOWS_SDK_VERSION="$(find_version "$WINDOWS_SDK_ROOT" lib um/x64/kernel32.lib)"
    
    if [ -z "$WINDOWS_SDK_VERSION" ]; then
      echo "No Windows 10 SDK Version found" >&2
      exit 1
    fi

    append_path PATH "$WINDOWS_KIT_ROOT/bin/$WINDOWS_KIT_VERSION/x64"
    append_path LIB_CYGPATH "$WINDOWS_KIT_ROOT/lib/$WINDOWS_KIT_VERSION/ucrt/x64"
    append_path INCLUDE_CYGPATH "$WINDOWS_KIT_ROOT/Include/$WINDOWS_KIT_VERSION/ucrt"

    append_path PATH "$WINDOWS_SDK_ROOT/bin/x64"
    append_path LIB_CYGPATH "$WINDOWS_SDK_ROOT/lib/$WINDOWS_SDK_VERSION/um/x64"
    append_path INCLUDE_CYGPATH "$WINDOWS_SDK_ROOT/Include/$WINDOWS_SDK_VERSION/um"
    append_path INCLUDE_CYGPATH "$WINDOWS_SDK_ROOT/Include/$WINDOWS_SDK_VERSION/cppwinrt"
    append_path INCLUDE_CYGPATH "$WINDOWS_SDK_ROOT/Include/$WINDOWS_SDK_VERSION/winrt"
    append_path INCLUDE_CYGPATH "$WINDOWS_SDK_ROOT/Include/$WINDOWS_SDK_VERSION/shared"

    LIB="$(cygpath -wp "$LIB_CYGPATH")"
    INCLUDE="$(cygpath -wp "$INCLUDE_CYGPATH")"
    export PATH
    export LIB
    export INCLUDE
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
