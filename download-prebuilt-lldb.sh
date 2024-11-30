#!/bin/sh

set -eu

. "$(dirname "$0")/build-lldb-info.sh"

RELEASE_NAME="lldb-${LLVM_COMMIT_SHORT}-${BUILD_CONFIG_VERSION}"

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

BUILT_URL="https://github.com/anatawa12/ConsoleLogSaver/releases/download/${RELEASE_NAME}/${RELEASE_NAME}-${OS}.tar.gz"

curl -L "$BUILT_URL" | tar xz
