using Anatawa12.ConsoleLogSaver;

var pid = int.Parse(args[0]);

var saver = new ConsoleLogSaver();
// TODO: configuration
Console.WriteLine(LogFileWriter.WriteToString(await saver.CollectFromPid(pid)));
