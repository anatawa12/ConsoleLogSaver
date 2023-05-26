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
