using Anatawa12.ConsoleLogSaver;

int? pidIn = null;
var saver = new ConsoleLogSaver();

foreach (var s in args)
{
    switch (s)
    {
        case "--hide-user-name":
            saver.HideUserName = true;
            break;
        case "--show-user-name":
            saver.HideUserName = false;
            break;
        case "--hide-user-home":
            saver.HideUserHome = true;
            break;
        case "--show-user-home":
            saver.HideUserHome = false;
            break;
        case "--hide-os-info":
            saver.HideOsInfo = true;
            break;
        case "--show-os-info":
            saver.HideOsInfo = false;
            break;
        case "--list":
            await FindProcesses();
            break;
        case "--help":
        case "-h":
            PrintHelp(0);
            break;
        default:
            pidIn = int.Parse(s);
            break;
    }
}

DebuggerSession session;

if (pidIn is { } pid)
{
    session = await DebuggerSession.Connect(pid);
}
else
{
    var process = await DebuggerSession.ConnectAllUnityProcesses(TimeSpan.FromSeconds(1));
    try
    {
        if (process.Length == 0)
            throw new Exception("No UnityEditors found");
        if (process.Length != 1)
            Console.Error.WriteLine(
                $"WARNING: Multiple Unity Editors found. using {process[0].Pid} for {process[1].ProjectRoot}");

        session = process[0];
    }
    catch
    {
        if (process.Length != 0)
            process[0].Dispose();
        throw;
    }
    finally
    {
        foreach (var debuggerSession in process.Skip(1)) debuggerSession.Dispose();
    }
}

Console.WriteLine(LogFileWriter.WriteToString(await saver.Collect(session)));

void PrintHelp(int exitCode)
{
    var process = Environment.GetCommandLineArgs()[0];
    Console.Error.WriteLine($"{process} [OPTIONS] <unity pid>");
    Console.Error.WriteLine("Experimental ConsoleLogSaver with mono debug protocol");
    Console.Error.WriteLine("");
    Console.Error.WriteLine("OPTIONS:");
    Console.Error.WriteLine("\t--hide-user-name: enable Hide User Name log filter");
    Console.Error.WriteLine("\t--show-user-name: disable Hide User Name log filter");
    Console.Error.WriteLine("\t--hide-user-home: enable Hide User Home log filter");
    Console.Error.WriteLine("\t--show-user-home: disable Hide User Home log filter");
    Console.Error.WriteLine("\t--hide-os-info: enable Hide OS Info flag");
    Console.Error.WriteLine("\t--show-os-info: disable Hide OS Info flag");
    Console.Error.WriteLine("\t--list: list unity processes and exit");
    Console.Error.WriteLine("\t--help: show this message and exit");
    Environment.Exit(exitCode);
}

async Task FindProcesses()
{
    var sessions = await DebuggerSession.ConnectAllUnityProcesses(TimeSpan.FromSeconds(1));
    foreach (var debuggerSession in sessions)
        Console.Error.WriteLine($"{debuggerSession.Pid} for {debuggerSession.ProjectRoot}");
    Environment.Exit(0);
}
