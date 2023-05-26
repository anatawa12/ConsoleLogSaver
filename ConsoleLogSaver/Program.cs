using System.Net;
using System.Reflection;
using Mono.Debugger.Soft;

var pid = int.Parse(args[0]);

var vm = VirtualMachineManager.Connect(new IPEndPoint(
    new IPAddress(stackalloc byte[] { 127, 0, 0, 1 }),
    56000 + pid % 1000));

if (vm == null)
{
    throw new Exception($"Cannot connect to pid {pid}");
}

Task<ThreadMirror> WaitForSuspend(VirtualMachine vm)
{
    return Task.Run(() =>
    {
        while (true)
        {
            var eventSet = vm.GetNextEventSet();
            foreach (var eventSetEvent in eventSet.Events)
            {
                if (eventSetEvent.EventType == EventType.Breakpoint)
                {
                    return eventSetEvent.Thread;
                }
            }
        }
    });
}

vm.SetBreakpoint(vm
    .GetTypes("UnityEditor.EditorApplication", false)
    .SelectMany(x => x.GetMethods())
    .First(x => x.Name == "Internal_CallUpdateFunctions"), 0);

var thread = await WaitForSuspend(vm);

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

try
{
    logEntries.SetConsoleFlag(ConsoleFlags.Collapse, false);
    logEntries.SetConsoleFlag(ConsoleFlags.LogLevelLog, true);
    logEntries.SetConsoleFlag(ConsoleFlags.LogLevelError, true);
    logEntries.SetConsoleFlag(ConsoleFlags.LogLevelWarning, true);
    
    
    Console.WriteLine($"logEntries.ConsoleFlags after: {logEntries.ConsoleFlags}");
    Console.WriteLine("================================================================");
    using var scope = new GettingLogEntriesScope(logEntries);

    var entry = LogEntryWrapper.New(logEntryTypeMirror, thread);
    for (var i = 0; i < scope.TotalRows; i++)
    {
        logEntries.GetEntryInternal(i, entry);
        var mode = entry.Mode;
        Console.WriteLine($"mode: {mode}");
        Console.WriteLine();
        Console.WriteLine(entry.Message);
        Console.WriteLine("================================================================");
        //var sectionBuilder = new Section.Builder("log-element");
        //sectionBuilder.AddField("Mode", ((Mode)mode).ToString());
        //sectionBuilder.AddField("Mode-Raw", $"{mode:x08}");
        //sectionBuilder.Content.Append(ReplaceMessage(entry.message));
        //fileBuilder.AddSection(sectionBuilder.Build());
    }
}
finally
{
    logEntries.ConsoleFlags = flags;
}

vm.Resume();
Console.WriteLine("Resumed The VM");
vm.Detach();

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

abstract class WrapperBase<T> where T : IInvokable
{
    public readonly T This;
    protected readonly TypeMirror Type;
    private readonly ThreadMirror Thread;
    private VirtualMachine VirtualMachine => Type.VirtualMachine;

    protected WrapperBase(TypeMirror type, T @this, ThreadMirror thread)
    {
        Type = type;
        This = @this;
        Thread = thread;
    }

    private PropertyInfoMirror Property(string name, ref PropertyInfoMirror? property)
    {
        if (property != null) return property;
        return property = Type.GetProperty(name)
                          ?? throw new InvalidOperationException($"Property {name} not found");
    }

    private MethodMirror Method(string name, string[] paramTypes, ref MethodMirror? method)
    {
        if (method != null) return method;
        foreach (var methodMirror in Type.GetMethods())
        {
            if (methodMirror.Name != name) goto continue_to;
            var parameters = methodMirror.GetParameters();
            if (parameters.Length != paramTypes.Length) goto continue_to;
            for (var i = 0; i < parameters.Length; i++)
                if (parameters[i].ParameterType.FullName != paramTypes[i])
                    goto continue_to;

            return method = methodMirror;
            
            continue_to: ;
        }
        throw new InvalidOperationException($"Method {name} not found");
    }

    protected Value PrimitiveValue(object value) => new PrimitiveValue(VirtualMachine, value);

    protected Value GetProperty(string name, ref PropertyInfoMirror? property) =>
        This.InvokeMethod(Thread, Property(name, ref property).GetGetMethod(true), Array.Empty<Value>());

    protected void SetProperty(string name, ref PropertyInfoMirror? property, Value value) =>
        This.InvokeMethod(Thread, Property(name, ref property).GetSetMethod(true), new[] { value });

    protected Value CallMethod(string name, string[] paramTypes, Value[] args, ref MethodMirror? method) =>
        This.InvokeMethod(Thread, Method(name, paramTypes, ref method), args);
}

abstract class ObjectWrapperBase : WrapperBase<ObjectMirror>
{
    protected ObjectWrapperBase(TypeMirror type, ObjectMirror @this, ThreadMirror thread) : base(type, @this, thread)
    {
    }

    private FieldInfoMirror Field(string name, ref FieldInfoMirror? field)
    {
        if (field != null) return field;
        return field = Type.GetField(name)
                          ?? throw new InvalidOperationException($"Field {name} not found");
    }

    protected Value GetField(string name, ref FieldInfoMirror? field) =>
        This.GetValue(Field(name, ref field));
}

class LogEntriesWrapper : WrapperBase<TypeMirror>
{
    public LogEntriesWrapper(TypeMirror type, ThreadMirror thread) : base(type, type, thread)
    {
    }

    private PropertyInfoMirror? _consoleFlags;
    private MethodMirror? _setConsoleFlag;
    private MethodMirror? _startGettingEntries;
    private MethodMirror? _endGettingEntries;
    private MethodMirror? _getEntryInternal;

    public ConsoleFlags ConsoleFlags
    {
        get => (ConsoleFlags)GetProperty("consoleFlags", ref _consoleFlags).AsInt32();
        set => SetProperty("consoleFlags", ref _consoleFlags, PrimitiveValue((int)value));
    }

    public void SetConsoleFlag(
        ConsoleFlags bit,
        bool value
    ) => CallMethod("SetConsoleFlag",
        new[] { "System.Int32", "System.Boolean" },
        new[] { PrimitiveValue((int)bit), PrimitiveValue((bool)value) },
        ref _setConsoleFlag);

    public int StartGettingEntries() => CallMethod("StartGettingEntries",
        Array.Empty<string>(),
        Array.Empty<Value>(),
        ref _startGettingEntries).AsInt32();

    public void EndGettingEntries() => CallMethod("EndGettingEntries",
        Array.Empty<string>(),
        Array.Empty<Value>(),
        ref _endGettingEntries);

    public void GetEntryInternal(int i, LogEntryWrapper entry) => CallMethod("GetEntryInternal",
        new[] { "System.Int32", "UnityEditor.LogEntry" },
        new[] { PrimitiveValue(i), entry.This },
        ref _getEntryInternal);
}

class LogEntryWrapper : ObjectWrapperBase
{
    public LogEntryWrapper(TypeMirror type, ObjectMirror value, ThreadMirror thread) : base(type, value, thread)
    {
    }

    private FieldInfoMirror? _mode;
    private FieldInfoMirror? _messsage;

    public Mode Mode => (Mode)GetField("mode", ref _mode).AsInt32();
    public string Message => GetField("message", ref _messsage).AsString();

    public static LogEntryWrapper New(TypeMirror logEntryType, ThreadMirror thread)
    {
        var ctor = logEntryType.GetMethodsByNameFlags(".ctor",
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
                false)
            .First(x => x.GetParameters().Length == 0);

        return new LogEntryWrapper(logEntryType,
            (ObjectMirror)logEntryType.NewInstance(thread, ctor, Array.Empty<Value>()), thread);
    }

}

static class ValueExtensions
{
    public static int AsInt32(this Value value) => (int)((PrimitiveValue)value).Value;
    public static string AsString(this Value value) => ((StringMirror)value).Value;
}

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
