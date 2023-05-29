using Mono.Debugger.Soft;

namespace Anatawa12.ConsoleLogSaver;

static class ValueExtensions
{
    public static int AsInt32(this Value value) => (int)((PrimitiveValue)value).Value;
    public static int AsInt32Enum(this Value value) => (int)((EnumMirror)value).Value;
    public static string AsString(this Value value) => ((StringMirror)value).Value;

    public static TypeMirror FindType(this VirtualMachine vm, string? assembly, string type) =>
        vm.GetTypes(type, false)
            .First(x => assembly == null || x.Assembly.GetName().Name == assembly);
}
