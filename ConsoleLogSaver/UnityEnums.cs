namespace Anatawa12.ConsoleLogSaver;

[Flags]
enum ConsoleFlags
{
    Collapse = 1 << 0,
    ClearOnPlay = 1 << 1,
    ErrorPause = 1 << 2,
    Verbose = 1 << 3,
    StopForAssert = 1 << 4,
    StopForError = 1 << 5,
    Autoscroll = 1 << 6,
    LogLevelLog = 1 << 7,
    LogLevelWarning = 1 << 8,
    LogLevelError = 1 << 9,
    ShowTimestamp = 1 << 10,
    ClearOnBuild = 1 << 11,
    ClearOnRecompile = 1 << 12,
    UseMonospaceFont = 1 << 13,
    StripLoggingCallstack = 1 << 14,
}

[Flags]
internal enum Mode
{
    Error = 1 << 0,
    Assert = 1 << 1,
    Log = 1 << 2,
    Fatal = 1 << 4,
    DontPreprocessCondition = 1 << 5,
    AssetImportError = 1 << 6,
    AssetImportWarning = 1 << 7,
    ScriptingError = 1 << 8,
    ScriptingWarning = 1 << 9,
    ScriptingLog = 1 << 10,
    ScriptCompileError = 1 << 11,
    ScriptCompileWarning = 1 << 12,
    StickyError = 1 << 13,
    MayIgnoreLineNumber = 1 << 14,
    ReportBug = 1 << 15,
    DisplayPreviousErrorInStatusBar = 1 << 16,
    ScriptingException = 1 << 17,
    DontExtractStacktrace = 1 << 18,
    ShouldClearOnPlay = 1 << 19,
    GraphCompileError = 1 << 20,
    ScriptingAssertion = 1 << 21,
    VisualScriptingError = 1 << 22
}

// ReSharper disable InconsistentNaming
public enum BuildTarget
{
    NoTarget = -2,
    BB10 = -1,
    MetroPlayer = -1,
    iPhone = -1,
    StandaloneOSX = 2,
    StandaloneOSXUniversal = 3,
    StandaloneOSXIntel = 4,
    StandaloneWindows = 5,
    WebPlayer = 6,
    WebPlayerStreamed = 7,
    iOS = 9,
    PS3 = 10,
    XBOX360 = 11,
    Android = 13,
    StandaloneLinux = 17,
    StandaloneWindows64 = 19,
    WebGL = 20,
    WSAPlayer = 21,
    StandaloneLinux64 = 24,
    StandaloneLinuxUniversal = 25,
    WP8Player = 26,
    StandaloneOSXIntel64 = 27,
    BlackBerry = 28,
    Tizen = 29,
    PSP2 = 30,
    PS4 = 31,
    PSM = 32,
    XboxOne = 33,
    SamsungTV = 34,
    N3DS = 35,
    WiiU = 36,
    tvOS = 37,
    Switch = 38,
    Lumin = 39,
    Stadia = 40,
    CloudRendering = 41,
    GameCoreXboxSeries = 42,
    GameCoreXboxOne = 43,
    PS5 = 44,
}
// ReSharper restore InconsistentNaming
