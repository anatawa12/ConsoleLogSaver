#!/bin/sh

set -eu

mkdir -p bin

for target in osx-arm64 osx-x64 win-x64; do
  dotnet publish ./ConsoleLogSaver/ConsoleLogSaver.csproj -r "$target" -c:Release
  PUBLISH="./ConsoleLogSaver/bin/Release/net6.0/$target/publish"
  test -f "$PUBLISH/ConsoleLogSaver" && cp "$PUBLISH/ConsoleLogSaver" "bin/ConsoleLogSaver-$target"
  test -f "$PUBLISH/ConsoleLogSaver.exe" && cp "$PUBLISH/ConsoleLogSaver.exe" "bin/ConsoleLogSaver-$target.exe"
  test -f "$PUBLISH/ConsoleLogSaver.pdb" && cp "$PUBLISH/ConsoleLogSaver.pdb" "bin/ConsoleLogSaver-$target.pdb"
done
