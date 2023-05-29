using System.Reflection;
using Mono.Debugger.Soft;

namespace Anatawa12.ConsoleLogSaver;

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

abstract class StaticWrapperBase : WrapperBase<TypeMirror>
{
    protected StaticWrapperBase(TypeMirror type, ThreadMirror thread) : base(type, type, thread)
    {
    }
    
    protected StaticWrapperBase(ThreadMirror thread, string? assembly, string type)
        : this(thread.VirtualMachine.FindType(assembly, type), thread)
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

class LogEntriesWrapper : StaticWrapperBase
{
    public LogEntriesWrapper(ThreadMirror thread) : base(thread, "UnityEditor", "UnityEditor.LogEntries")
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

class ApplicationWrapper : StaticWrapperBase
{
    public ApplicationWrapper(ThreadMirror thread) : base(thread, "UnityEngine.CoreModule", "UnityEngine.Application")
    {
    }

    private PropertyInfoMirror? _unityVersion;

    public string UnityVersion => GetProperty("unityVersion", ref _unityVersion).AsString();
}

class EditorUserBuildSettingsWrapper : StaticWrapperBase
{
    public EditorUserBuildSettingsWrapper(ThreadMirror thread) : base(thread, "UnityEditor", "UnityEditor.EditorUserBuildSettings")
    {
    }

    private PropertyInfoMirror? _unityVersion;

    public BuildTarget ActiveBuildTarget => (BuildTarget)GetProperty("activeBuildTarget", ref _unityVersion).AsInt32Enum();
}

class DirectoryWrapper : StaticWrapperBase
{
    public DirectoryWrapper(ThreadMirror thread) : base(thread, null, "System.IO.Directory")
    {
    }

    private MethodMirror? _getCurrentDirectory;

    public string GetCurrentDirectory() => CallMethod("GetCurrentDirectory",
        Array.Empty<string>(),
        Array.Empty<Value>(),
        ref _getCurrentDirectory).AsString();
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

    public static LogEntryWrapper New(ThreadMirror thread)
    {
        var logEntryType = thread.VirtualMachine.FindType("UnityEditor", "UnityEditor.LogEntry");
        var ctor = logEntryType.GetMethodsByNameFlags(".ctor",
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
                false)
            .First(x => x.GetParameters().Length == 0);

        return new LogEntryWrapper(logEntryType,
            (ObjectMirror)logEntryType.NewInstance(thread, ctor, Array.Empty<Value>()), thread);
    }

}
