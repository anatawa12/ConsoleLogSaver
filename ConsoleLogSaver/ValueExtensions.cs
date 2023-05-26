using Mono.Debugger.Soft;

namespace Anatawa12.ConsoleLogSaver;

static class ValueExtensions
{
    public static int AsInt32(this Value value) => (int)((PrimitiveValue)value).Value;
    public static string AsString(this Value value) => ((StringMirror)value).Value;
    
    public static Task<ThreadMirror> WaitForBreakPoint(this VirtualMachine vm)
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
}
