using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using Mono.Debugger.Soft;

namespace Anatawa12.ConsoleLogSaver;

public partial class ConsoleLogSaver
{
    public bool HideOsInfo = false;
    public bool HideUserName = true;
    public bool HideUserHome = true;
    public bool HideAwsUploadSignature = true;

    private Regex? _homePatternRegex;

    private Regex? HomePatternRegex
    {
        get
        {
            if (!HideUserHome) return null;
            if (_homePatternRegex != null) return _homePatternRegex;

            var homePath = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            var pathComponents = homePath.Split('/', '\\');
            return _homePatternRegex = new Regex(string.Join("[\\\\/]", pathComponents.Select(Regex.Escape)),
                RegexOptions.IgnoreCase);
        }
    }

    private Regex? _namePatternRegex;

    private Regex? NamePatternRegex
    {
        get
        {
            if (!HideUserName) return null;
            if (_namePatternRegex != null) return _namePatternRegex;

            return _namePatternRegex = new Regex(Regex.Escape(Environment.UserName), RegexOptions.IgnoreCase);
        }
    }

    [GeneratedRegex(@"(?<=AWSAccessKeyId=)[^&\s]+", RegexOptions.IgnoreCase)]
    private static partial Regex AwsAccessKeyIdGetParameterRegex();

    [GeneratedRegex(@"(?<=Signature=)[^&\s]+", RegexOptions.IgnoreCase)]
    private static partial Regex SignatureGetParameterRegex();

    [GeneratedRegex(
        """
        ("assetUrl"\s*:\s*")((?:[^\u0000-\u001F"\\]|\\(?:u[a-fA-F0-9]{4}|[^"\\/bfnrt]))*)(")
        """
    )]
    private static partial Regex AssetUrlRegex();

    public async Task<ConsoleLogFileV1> Collect(DebuggerSession session)
    {
        using var scope = await session.WaitAndRunInMainThread();
        var thread = scope.Thread;

        var fileBuilder = new ConsoleLogFileV1.Builder(0);

        var application = new ApplicationWrapper(thread);
        var editorUserBuildSettings = new EditorUserBuildSettingsWrapper(thread);
        var projectRoot = new DirectoryWrapper(thread).GetCurrentDirectory();
        // header
        fileBuilder.AddField("Vendor", "ConsoleLogSaver/" + CheckForUpdate.CurrentVersion);
        fileBuilder.AddField("Unity-Version", application.UnityVersion);
        fileBuilder.AddField("Build-Target", editorUserBuildSettings.ActiveBuildTarget.ToString());
        if (!HideOsInfo) fileBuilder.AddField("Editor-Platform", RuntimeInformation.OSDescription);
        if (HideUserName) fileBuilder.AddField("Hidden-Data", "user-name");
        if (HideUserHome) fileBuilder.AddField("Hidden-Data", "user-home");
        fileBuilder.AddField("Hidden-Data", "aws-access-key-id-param");
        fileBuilder.AddField("Hidden-Data", "asset-url");
        if (HideAwsUploadSignature) fileBuilder.AddField("Hidden-Data", "signature-param");
        AppendUpm(fileBuilder, projectRoot);
        AppendVpm(fileBuilder, projectRoot);
        AppendLog(thread, fileBuilder);

        return fileBuilder.Build();
    }

    public async Task<String> CollectStackTrace(DebuggerSession session)
    {
        var vm = session.VirtualMachine;

        // first, we suspend VM to get stack trace
        vm.Suspend();

        var builder = new StringBuilder();
        ThreadMirror.NativeTransitions = true;

        foreach (var threadMirror in vm.GetThreads())
        {
            builder.Append($"Thread '{threadMirror.Name}' (system tid: {threadMirror.TID}, managed tid: {threadMirror.ThreadId}):\n");


            foreach (var stackFrame in threadMirror.GetFrames())
                builder.Append($"  {stackFrame.Location}\n");
            builder.Append("\n");
        }

        return builder.ToString();
    }

    private void AppendLog(ThreadMirror thread, ConsoleLogFileV1.Builder fileBuilder)
    {
        var logEntries = new LogEntriesWrapper(thread);

        var flags = logEntries.ConsoleFlags;
        try
        {
            logEntries.SetConsoleFlag(ConsoleFlags.Collapse, false);
            logEntries.SetConsoleFlag(ConsoleFlags.LogLevelLog, true);
            logEntries.SetConsoleFlag(ConsoleFlags.LogLevelError, true);
            logEntries.SetConsoleFlag(ConsoleFlags.LogLevelWarning, true);

            using var scope = new GettingLogEntriesScope(logEntries);

            var entry = LogEntryWrapper.New(thread);
            for (var i = 0; i < scope.TotalRows; i++)
            {
                logEntries.GetEntryInternal(i, entry);
                var mode = entry.Mode;
                var sectionBuilder = new Section.Builder("log-element");
                sectionBuilder.AddField("Mode", mode.ToString());
                sectionBuilder.AddField("Mode-Raw", $"{(int)mode:x08}");
                sectionBuilder.Content.Append(ReplaceMessage(entry.Message));
                fileBuilder.AddSection(sectionBuilder.Build());
            }
        }
        finally
        {
            logEntries.ConsoleFlags = flags;
        }
    }

    private void AppendUpm(ConsoleLogFileV1.Builder builder, string projectRoot)
    {
        foreach (var (package, type, version) in PackageManagerInfoCollector.UpmLockedPackages(projectRoot))
        {
            bool needsReplace;
            switch (type)
            {
                case UpmDependencyType.Upm:
                case UpmDependencyType.HttpsGit:
                case UpmDependencyType.SshGit:
                case UpmDependencyType.GitGit:
                    // it's a remote one: It's very rarely to have personal info in version name.
                    needsReplace = false;
                    break;

                case UpmDependencyType.FileGit:
                    // It's likely to have personal info in absolute paths so hide it
                    needsReplace = true;
                    break;

                case UpmDependencyType.FileRelative:
                    // It's rarely to have personal info in relative paths.
                    needsReplace = version.StartsWith("file:../..", StringComparison.Ordinal)
                                   || version.StartsWith("file:..\\..", StringComparison.Ordinal);
                    break;
                case UpmDependencyType.FileAbsolute:
                    // It's likely to have personal info in absolute paths so hide it
                    needsReplace = true;
                    break;
                default:
                    throw new ArgumentOutOfRangeException();
            }

            builder.AddField("Upm-Dependency", $"{package}@{(needsReplace ? ReplaceMessage(version) : version)}");
        }
    }

    private void AppendVpm(ConsoleLogFileV1.Builder builder, string projectRoot)
    {
        foreach (var (package, version) in PackageManagerInfoCollector.VpmLockedPackages(projectRoot))
        {
            // for vpm dependency, everything including local packages are identified using package id so
            // it's not likely to include personal info.
            builder.AddField("Vpm-Dependency", $"{package}@{(version)}");
        }
    }

    private string ReplaceMessage(string str)
    {
        str = AwsAccessKeyIdGetParameterRegex().Replace(str, "${aws-access-key-id-param}");
        str = AssetUrlRegex().Replace(str, m => m.Groups[1].Value + "${asset-url}" + m.Groups[3].Value);
        if (HideAwsUploadSignature)
            str = SignatureGetParameterRegex().Replace(str, "${signature-param}");
        if (HomePatternRegex is { } homePatternRegex)
            str = homePatternRegex.Replace(str, "${user-home}");
        if (NamePatternRegex is { } namePatternRegex)
            str = namePatternRegex.Replace(str, "${user-name}");
        return str;
    }

    internal struct GettingLogEntriesScope : IDisposable
    {
        private LogEntriesWrapper? _logEntries;
        public readonly int TotalRows;

        public GettingLogEntriesScope(LogEntriesWrapper logEntries)
        {
            _logEntries = logEntries;
            TotalRows = logEntries.StartGettingEntries();
        }

        public void Dispose()
        {
            _logEntries?.EndGettingEntries();
            _logEntries = null;
        }
    }
}
