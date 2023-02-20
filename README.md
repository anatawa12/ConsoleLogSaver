Compile Log Saver
===

The tool to share Unity console log with friend who is good to error resolving.

エラー解決に強い友達とUnityのコンソールログを共有するためのツール

How to Use
---

1. Download latest unitypackage from [here][saver-unitypackage-download]
2. import to your Unity project
3. Open window at `Tools/Console Log Saver`
4. Click `Save to File` & shere created file \
   OR `Copy to Clipboard` and share using something like [pastebin.com]

使い方
---
1. 最新版のunitypackageを[ここ][saver-unitypackage-download]からダウンロードする
2. Unityプロジェクトにimportする
3. `Tools/Console Log Saver`を選択してウィンドウを開く
4. `Save to File`を押して生成されたファイルを共有する\
   または`Copy to Clipboard`して[pastebin.com]などで共有する。

File Format
---

The exported file is designed as human-readable & machine-readable.
This section shows how to read the file for humans. Doc for for creating parser is not yet provided.

The document is consists of multiple parts. First section as header (header section) and others are log content (content section).

For each section, there's header fields like `HTTP/1.1`'s one and contents after two new lines (CRLF or LF).

In the header section, content should not be exists and you should ignore contents.

In the header section, there is a required field.

- `Separator: ` shows the separator for each section. except for header section, the separator followed by new line (CRLF or LF) should not be exists in the both header fields and content.

Also, header section may have the following optional field.

- `Unity-Version: ` The Unity Editor versionログの発生したUnityのバージョン
- `Build-Target: ` The [current build target][unity-build-target] ログを収集した時点でのビルド対象
- `Editor-Platform: ` The OS information of the Unity Editor
- `Hidden-Data: ` The data may be hidden (replaced with some text) in the log

In each content section, there is a required field.

- `Content: ` shows the type of content. currently `log-element` is only used.

Also, content section with `Content: log-element` will have the following required fields

- `Mode: ` The metadata of the log element. list of name of high bit.
- `Mode-Raw: ` The metadata of the log element in hex.

ファイルフォーマット
---

生成されたファイルは機械でも人間でも判読可能に設計されてます。この章ではどのように人間がファイルを読めばいいかを示しています。パーサを書くためのドキュメントは用意されてません。

ドキュメントは複数の section に分けられており、最初のsectionがヘッダー(header section)で、残りがログの内容です(content section).

それぞれの section では `HTTP/1.1`と同様のヘッダフィールドが先頭にあり、2つの改行(CRLFまたはLF)の後、セクションの内容があります。

header sectionでは内容は空であるべきで、もしあっても無視するべきです。

header sectionでは以下の必須なフィールドがあります。

- `Separator: ` section の区切りを示します。header section を除き、各セクションにはこの区切りに改行(CRLFまたはLF)が続くものは含まれてはいけません。

また、 header section では以下の任意のフィールドがあります。

- `Unity-Version: ` ログの発生したUnityのバージョン
- `Build-Target: ` ログを収集した時点での[ビルド対象][unity-build-target]
- `Editor-Platform: ` UnityEditorを実行している環境
- `Hidden-Data: ` ログの内容で隠されてる可能性のある情報

各 content section では以下の必須なフィールドがあります。

- `Content: ` 内容の種別を示します。 `log-element` のみが使用されてます

`Content: log-element`なcontent section では以下の必須なフィールドがあります。

- `Mode: ` そのログの要素のメタデータ。1になっているビットの名前の羅列
- `Mode-Raw: ` そのログの要素のメタデータの16進数表記

[saver-unitypackage-download]: https://github.com/anatawa12/CompileLogSaver/releases/latest/download/saver.unitypackage
[pastebin.com]: https://pastebin.com/
[unity-build-target]: https://docs.unity3d.com/2021.2/ScriptReference/EditorUserBuildSettings-activeBuildTarget.html

