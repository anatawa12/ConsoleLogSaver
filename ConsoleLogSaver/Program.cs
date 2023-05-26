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
        default:
            pidIn = int.Parse(s);
            break;
    }
}

if (pidIn is not { } pid)
    throw new Exception("NO PID PROVIDED");

Console.WriteLine(LogFileWriter.WriteToString(await saver.CollectFromPid(pid)));
