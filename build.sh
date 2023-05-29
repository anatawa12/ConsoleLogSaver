#!/bin/sh

set -eu

rm -rf bin
mkdir -p bin

# utils

cp_if() {
  test -f "$1" && cp "$1" "$2" || :
}

# build
build_cli() {
  for target in osx-arm64 osx-x64 win-x64; do
    dotnet publish ./ConsoleLogSaver.Cli/ConsoleLogSaver.Cli.csproj -r "$target" -c:Release
    PUBLISH="./ConsoleLogSaver.Cli/bin/Release/net6.0/$target/publish"
    cp_if "$PUBLISH/ConsoleLogSaver.Cli"     "bin/ConsoleLogSaver.Cli-$target"
    cp_if "$PUBLISH/ConsoleLogSaver.Cli.exe" "bin/ConsoleLogSaver.Cli-$target.exe"
  done
}

build_gui() {
  target=win-x64
  PUBLISH="./ConsoleLogSaver.Gui/bin/Release/net6.0-windows/$target/publish"
  rm -rf "${PUBLISH:?}/bin" "${PUBLISH:?}/ConsoleLogSaver.Gui.zip"

  dotnet publish ./ConsoleLogSaver.Gui/ConsoleLogSaver.Gui.csproj -c:Release
  # because of https://github.com/dotnet/runtime/issues/3828, we need to fix subsystem of PE file
  dotnet run --project PEFlagSetter "$PUBLISH/ConsoleLogSaver.Gui.exe"

  mkdir -p "$PUBLISH/bin"
  mv "$PUBLISH"/*.exe "$PUBLISH"/*.dll "$PUBLISH/bin"
  ( cd "$PUBLISH/bin" && zip -r ../ConsoleLogSaver.Gui.zip ./*.dll ./*.exe)

  cp "$PUBLISH/ConsoleLogSaver.Gui.zip" "bin/ConsoleLogSaver.Gui-$target.zip"
}

build_cli
build_gui
