using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(Types.Exporter.ExporterGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(Types.TypesGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
