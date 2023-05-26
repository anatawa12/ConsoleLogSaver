using System.Net;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;
using Mono.Debugger.Soft;

namespace Anatawa12.ConsoleLogSaver;

public class ConsoleLogSaver
{
    public bool HideOsInfo = false;
    public bool HideUserName = true;
    public bool HideUserHome = true;

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

    internal async Task<ConsoleLogFileV1> CollectFromPid(int pid)
    {
        var vm = VirtualMachineManager.Connect(new IPEndPoint(
            new IPAddress(stackalloc byte[] { 127, 0, 0, 1 }),
            56000 + pid % 1000));

        if (vm == null)
        {
            throw new Exception($"Cannot connect to pid {pid}");
        }

        vm.SetBreakpoint(vm
            .GetTypes("UnityEditor.EditorApplication", false)
            .SelectMany(x => x.GetMethods())
            .First(x => x.Name == "Internal_CallUpdateFunctions"), 0);

        var thread = await vm.WaitForBreakPoint();

        var fileBuilder = new ConsoleLogFileV1.Builder();

        var application = new ApplicationWrapper(thread);
        var editorUserBuildSettings = new EditorUserBuildSettingsWrapper(thread);
        // header
        fileBuilder.AddField("Unity-Version", application.UnityVersion);
        fileBuilder.AddField("Build-Target", editorUserBuildSettings.ActiveBuildTarget.ToString());
        if (!HideOsInfo) fileBuilder.AddField("Editor-Platform", RuntimeInformation.OSDescription);
        if (HideUserName) fileBuilder.AddField("Hidden-Data", "user-name");
        if (HideUserHome) fileBuilder.AddField("Hidden-Data", "user-home");
        AppendUpm(fileBuilder);
        AppendVpm(fileBuilder);
        AppendLog(thread, fileBuilder);

        vm.Resume();
        vm.Detach();

        return fileBuilder.Build();
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

    private void AppendUpm(ConsoleLogFileV1.Builder builder)
    {
        // foreach (var (package, type, version) in PackageManagerInfoCollector.UpmLockedPackages())
        // {
        //     bool needsReplace;
        //     switch (type)
        //     {
        //         case UpmDependencyType.Upm:
        //         case UpmDependencyType.HttpsGit:
        //         case UpmDependencyType.SshGit:
        //         case UpmDependencyType.GitGit:
        //             // it's a remote one: It's very rarely to have personal info in version name.
        //             needsReplace = false;
        //             break;
        //
        //         case UpmDependencyType.FileGit:
        //             // It's likely to have personal info in absolute paths so hide it
        //             needsReplace = true;
        //             break;
        //
        //         case UpmDependencyType.FileRelative:
        //             // It's rarely to have personal info in relative paths.
        //             needsReplace = version.StartsWith("file:../..", StringComparison.Ordinal)
        //                            || version.StartsWith("file:..\\..", StringComparison.Ordinal);
        //             break;
        //         case UpmDependencyType.FileAbsolute:
        //             // It's likely to have personal info in absolute paths so hide it
        //             needsReplace = true;
        //             break;
        //         default:
        //             throw new ArgumentOutOfRangeException();
        //     }
        //
        //     builder.AddField("Upm-Dependency", $"{package}@{(needsReplace ? ReplaceMessage(version) : version)}");
        // }
    }

    private void AppendVpm(ConsoleLogFileV1.Builder builder)
    {
        // foreach (var (package, version) in PackageManagerInfoCollector.VpmLockedPackages())
        // {
        //     // for vpm dependency, everything including local packages are identified using package id so
        //     // it's not likely to include personal info.
        //     builder.AddField("Vpm-Dependency", $"{package}@{(version)}");
        // }
    }

    private string ReplaceMessage(string str)
    {
        if (HomePatternRegex is {} homePatternRegex)
            str = homePatternRegex.Replace(str, "${user-home}");
        if (NamePatternRegex is {} namePatternRegex)
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
