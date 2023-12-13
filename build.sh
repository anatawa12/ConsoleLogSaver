#!/bin/sh

set -eu

rm -rf bin
mkdir -p bin

FRAMEWORK=net8.0

# utils

cp_if() {
  test -f "$1" && cp "$1" "$2" || :
}

# build
build_cli() {
  for target in osx-arm64 osx-x64 win-x64 linux-x64; do
    dotnet publish ./ConsoleLogSaver.Cli/ConsoleLogSaver.Cli.csproj -r "$target" -c:Release
    PUBLISH="./ConsoleLogSaver.Cli/bin/Release/$FRAMEWORK/$target/publish"
    cp_if "$PUBLISH/ConsoleLogSaver.Cli"     "bin/ConsoleLogSaver.Cli-$target"
    cp_if "$PUBLISH/ConsoleLogSaver.Cli.exe" "bin/ConsoleLogSaver.Cli-$target.exe"
  done
}

build_gui() {
  target=win-x64
  PUBLISH="./ConsoleLogSaver.Gui/bin/Release/$FRAMEWORK-windows/$target/publish"
  rm -rf "${PUBLISH:?}/bin" "${PUBLISH:?}/ConsoleLogSaver.Gui.zip"

  dotnet publish ./ConsoleLogSaver.Gui/ConsoleLogSaver.Gui.csproj -c:Release

  mkdir -p "$PUBLISH/bin"
  mv "$PUBLISH"/*.exe "$PUBLISH"/*.dll "$PUBLISH/bin"
  ( cd "$PUBLISH/bin" && zip -r ../ConsoleLogSaver.Gui.zip ./*.dll ./*.exe)

  cp "$PUBLISH/ConsoleLogSaver.Gui.zip" "bin/ConsoleLogSaver.Gui-$target.zip"
}

build_cli
build_gui
