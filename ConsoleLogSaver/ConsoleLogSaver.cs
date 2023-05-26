using System.Net;
using Mono.Debugger.Soft;

namespace Anatawa12.ConsoleLogSaver;

public class ConsoleLogSaver
{
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

        Console.WriteLine("Suspended The VM");

        var logEntriesTypeMirror = vm
            .GetTypes("UnityEditor.LogEntries", false)
            .First(x => x.Assembly.GetName().Name?.Contains("UnityEditor") ?? false);
        var logEntryTypeMirror = vm
            .GetTypes("UnityEditor.LogEntry", false)
            .First(x => x.Assembly.GetName().Name?.Contains("UnityEditor") ?? false);

        var logEntries = new LogEntriesWrapper(logEntriesTypeMirror, thread);

        var flags = logEntries.ConsoleFlags;
        Console.WriteLine($"logEntries.ConsoleFlags: {flags}");

        var fileBuilder = new ConsoleLogFileV1.Builder();

        try
        {
            logEntries.SetConsoleFlag(ConsoleFlags.Collapse, false);
            logEntries.SetConsoleFlag(ConsoleFlags.LogLevelLog, true);
            logEntries.SetConsoleFlag(ConsoleFlags.LogLevelError, true);
            logEntries.SetConsoleFlag(ConsoleFlags.LogLevelWarning, true);

            using var scope = new GettingLogEntriesScope(logEntries);

            var entry = LogEntryWrapper.New(logEntryTypeMirror, thread);
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

        vm.Resume();
        vm.Detach();

        return fileBuilder.Build();
    }

    string ReplaceMessage(string str) => str; // TODO: do extraction

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
