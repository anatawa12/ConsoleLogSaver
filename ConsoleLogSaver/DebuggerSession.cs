using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;
using Mono.Debugger.Soft;

namespace Anatawa12.ConsoleLogSaver;

public class DebuggerSession : IDisposable
{
    private readonly int _pid;
    private readonly int _port;
    private VirtualMachine? _vm;
    private string _projectRoot = default!;

    public VirtualMachine VirtualMachine => _vm ?? throw new ObjectDisposedException("disposed");
    public string ProjectRoot => _projectRoot ?? throw new InvalidOperationException("bad state");
    public int Pid => _pid;
    public int Port => _port;

    private DebuggerSession(int pid, int port)
    {
        _pid = pid;
        _port = port;
    }

    public static async Task<DebuggerSession> Connect(int pid, CancellationToken cancellationToken = default)
    {
        var ports = new[]
        {
            56000 + pid % 1000,
            18000 + pid % 1000,
        };

        using var ourFinishTokenSource = new CancellationTokenSource();
        using var linkedSource = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, ourFinishTokenSource.Token);
        var linkedToken = linkedSource.Token;
            
        var complement = new TaskCompletionSource<DebuggerSession>();

        var connectTasks = new Task[ports.Length];
        for (var i = 0; i < connectTasks.Length; i++)
        {
            var port = ports[i];
            connectTasks[i] = Task.Run(async () =>
            {
                var session = await ConnectInternal(pid, port, linkedToken);
                if (!complement.TrySetResult(session))
                    session.Dispose();
            }, linkedToken);
        }

        var allTask = Task.WhenAll(connectTasks);

        var connectedTask = complement.Task;
        var task = await Task.WhenAny(allTask, connectedTask);

        ourFinishTokenSource.CancelAfter(TimeSpan.Zero);

        if (task == connectedTask)
        {
            return connectedTask.Result;
        }

        throw allTask.Exception!;
    }

    public static async Task<DebuggerSession> ConnectByPort(int port, CancellationToken cancellationToken = default) =>
        await ConnectInternal(-1, port, cancellationToken).ConfigureAwait(false);

    private static async Task<DebuggerSession> ConnectInternal(int pid, int port, CancellationToken cancellationToken = default)
    {
        var session = new DebuggerSession(pid, port);
        try
        {
            await session.DoConnect(cancellationToken);
        }
        catch (Exception e)
        {
            session.Dispose();

            if (e is OperationCanceledException cancell && cancell.CancellationToken == cancellationToken)
                throw;
            else
                throw new IOException($"Cannot connect to process (trying {port} for {pid})", e);
        }

        return session;
    }

    public static int[] FindUnityProcess()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
        {
            // For linux, Process.GetProcessesByName("Unity") does not work since
            // process name seen from net 6.0 / 7.0 is `Main Thread`.
            // Therefore, use `/proc/{pid}/cmdline` to get process name instead.
            // see https://github.com/anatawa12/ConsoleLogSaver/issues/21
            
            return Process.GetProcesses()
                .Where(p =>
                {
                    try
                    {
                        return ProcessPathLooksUnity(File.ReadAllBytes($"/proc/{p.Id}/cmdline"));
                    }
                    catch
                    {
                        // might be the process has been terminated
                        return false;
                    }
                })
                .Select(x => x.Id)
                .ToArray();
        }
        else
        {
            return Process.GetProcessesByName("Unity").Select(x => x.Id).ToArray();
        }
    }

    private static bool ProcessPathLooksUnity(ReadOnlySpan<byte> cmdLine)
    {
        var nullAt = cmdLine.IndexOf((byte)0);
        var length = nullAt != -1 ? nullAt : cmdLine.Length;
        return cmdLine[..length].EndsWith("Unity"u8);
    }

    public static async Task<DebuggerSession[]> ConnectAllUnityProcesses(TimeSpan connectTimeout)
    {
        var processes = FindUnityProcess();
        using var source = new CancellationTokenSource();
        var token = source.Token;
        var sessions = new DebuggerSession?[processes.Length];
        var tasks = new Task[processes.Length];
        for (var i = 0; i < processes.Length; i++)
        {
            var j = i;
            // ReSharper disable once MethodSupportsCancellation
            tasks[i] = Task.Run(async () => { sessions[j] = await Connect(processes[j], token); }, token);
        }

        source.CancelAfter(connectTimeout);

        try
        {
            try
            {
                await Task.WhenAll(tasks);
            }
            catch (AggregateException)
            {
                // ignored
            }
            catch (OperationCanceledException e) when (e.CancellationToken == token)
            {
                // ignored
            }
        }
        catch
        {
            foreach (var session in sessions)
                session?.Dispose();
        }

        // !: checked for is not null
        return sessions.Where(x => x is not null).ToArray()!;
    }

    private async Task DoConnect(CancellationToken cancellationToken = default)
    {
        _vm = await ConnectToVirtualMachine(new IPEndPoint(
            new IPAddress(stackalloc byte[] { 127, 0, 0, 1 }),
            _port));
        
        if (_vm == null) throw new IOException($"Cannot connect to process");

        using var scope = await WaitAndRunInMainThread(cancellationToken);
        var thread = scope.Thread;

        _projectRoot = new DirectoryWrapper(thread).GetCurrentDirectory();
    }

    public async Task<InThreadScope> WaitAndRunInMainThread(CancellationToken cancellationToken = default) =>
        new(await WaitForTick(cancellationToken));

    public class InThreadScope : IDisposable
    {
        public ThreadMirror Thread;

        internal InThreadScope(ThreadMirror thread)
        {
            Thread = thread;
        }

        public void Dispose()
        {
            Thread.VirtualMachine.Resume();
        }
    }

    private async Task<ThreadMirror> WaitForTick(CancellationToken cancellationToken = default)
    {
        if (_vm == null) throw new ObjectDisposedException("disposed");
        var method = _vm
            .GetTypes("UnityEditor.EditorApplication", false)
            .SelectMany(x => x.GetMethods())
            .First(x => x.Name == "Internal_CallUpdateFunctions");
        var breakpoint = _vm.SetBreakpoint(method, 0);
        ThreadMirror thread;

        try
        {
            thread = await Task.Run(() =>
            {
                while (true)
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    var eventSet = _vm.GetNextEventSet(100);
                    if (eventSet == null) continue;
                    foreach (var eventSetEvent in eventSet.Events)
                        if (eventSetEvent.EventType == EventType.Breakpoint)
                            return eventSetEvent.Thread;
                }
            }, cancellationToken);
        }
        finally
        { 
            breakpoint.Disable();
        }

        return thread;
    }

    private static Task<VirtualMachine> ConnectToVirtualMachine(IPEndPoint endpoint)
    {
        return Task<VirtualMachine>.Factory.FromAsync(
            (arg1, callback, _) => VirtualMachineManager.BeginConnect(arg1, callback),
            VirtualMachineManager.EndConnect, endpoint, null);
    }

    ~DebuggerSession()
    {
        Dispose(false);
    }

    public void Dispose()
    {
        GC.SuppressFinalize(this);
        Dispose(true);
    }

    public void Dispose(bool disposing)
    {
        if (_vm != null)
        {
            var vm = _vm;
            _vm = null;
            vm.Detach();
        }
    }
}
