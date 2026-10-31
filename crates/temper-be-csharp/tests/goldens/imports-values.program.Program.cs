using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(Values.Exporter.ExporterGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(Values.ValuesGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
