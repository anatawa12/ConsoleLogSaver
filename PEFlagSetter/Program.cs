using System.Buffers.Binary;
using System.IO.MemoryMappedFiles;

const int bufSize = 1024;
const int peMagic = 0x00004550; // "PE\0\0"
const int offsetOfSubsystem = 0x5C;
const short windowsGuiSubsystem = 0x2;
const short windowsCuiSubsystem = 0x3;

if (args.Length == 0) throw new Exception("no files specified");

foreach (var fileName in args)
{
    using var mmf = MemoryMappedFile.CreateFromFile(fileName, FileMode.Open);
    using var accessor = mmf.CreateViewAccessor(0, bufSize);
    var offset = accessor.ReadInt32LittleEndian(0x3c);
    if (offset + offsetOfSubsystem + sizeof(short) >= bufSize) throw new Exception("offset too big");
    if (accessor.ReadInt32LittleEndian(offset) != peMagic) throw new Exception("invalid magic");

    var subsystem = accessor.ReadInt16LittleEndian(offset + offsetOfSubsystem);
    switch (subsystem)
    {
        case windowsGuiSubsystem:
            Console.Error.WriteLine($"already gui: {fileName}");
            break;
        case windowsCuiSubsystem:
            accessor.WriteInt16LittleEndian(offset, windowsGuiSubsystem);
            Console.Error.WriteLine($"fixed subsystem: {fileName}");
            break;
        default:
            throw new Exception($"unknown subsystem: {subsystem:x04}");
    }
}

static class Extensions
{
    public static int ReadInt32LittleEndian(this UnmanagedMemoryAccessor memoryAccessor, int offset)
    {
        return BitConverter.IsLittleEndian ? memoryAccessor.ReadInt32(offset)
            : BinaryPrimitives.ReverseEndianness(memoryAccessor.ReadInt32(offset));
    }
    public static short ReadInt16LittleEndian(this UnmanagedMemoryAccessor memoryAccessor, int offset)
    {
        return BitConverter.IsLittleEndian ? memoryAccessor.ReadInt16(offset)
            : BinaryPrimitives.ReverseEndianness(memoryAccessor.ReadInt16(offset));
    }
    
    public static void WriteInt16LittleEndian(this UnmanagedMemoryAccessor memoryAccessor, int offset, short value)
    {
        memoryAccessor.Write(offset, BitConverter.IsLittleEndian ? value : BinaryPrimitives.ReverseEndianness(value));
    }
}
