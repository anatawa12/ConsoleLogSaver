// See https://aka.ms/new-console-template for more information

using System.Net;
using Mono.Debugger.Soft;

Console.WriteLine("Hello, World!");

var pid = int.Parse(args[0]);

var vm = VirtualMachineManager.Connect(new IPEndPoint(
    new IPAddress(stackalloc byte[] { 127, 0, 0, 1 }),
    56000 + pid % 1000));

if (vm == null)
{
    throw new Exception($"Cannot connect to pid {pid}");
}

vm.Suspend();
Console.WriteLine("Suspended The VM");
Thread.Sleep(1000 * 10);
vm.Resume();
Console.WriteLine("Resumed The VM");
vm.Detach();
