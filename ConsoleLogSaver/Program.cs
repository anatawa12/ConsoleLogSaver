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
        case "--help":
        case "-h":
            PrintHelp(0);
            break;
        default:
            pidIn = int.Parse(s);
            break;
    }
}

if (pidIn is not { } pid)
{
    var process = ConsoleLogSaver.FindUnityProcess();
    if (process.Length == 0)
        throw new Exception("No UnityEditors found");
    if (process.Length != 1)
        Console.Error.WriteLine($"WARNING: Multiple Unity Editors found. using {process[0]}");

    pid = process[0];
}

Console.WriteLine(LogFileWriter.WriteToString(await saver.CollectFromPid(pid)));

void PrintHelp(int exitCode)
{
    Console.Error.WriteLine("ConsoleLogSaver [OPTIONS] <unity pid>");
    Console.Error.WriteLine("Experimental ConsoleLogSaver with mono debug protocol");
    Console.Error.WriteLine("");
    Console.Error.WriteLine("OPTIONS:");
    Console.Error.WriteLine("\t--hide-user-name: enable Hide User Name log filter");
    Console.Error.WriteLine("\t--show-user-name: disable Hide User Name log filter");
    Console.Error.WriteLine("\t--hide-user-home: enable Hide User Home log filter");
    Console.Error.WriteLine("\t--show-user-home: disable Hide User Home log filter");
    Console.Error.WriteLine("\t--hide-os-info: enable Hide OS Info flag");
    Console.Error.WriteLine("\t--show-os-info: disable Hide OS Info flag");
    Environment.Exit(exitCode);
}
