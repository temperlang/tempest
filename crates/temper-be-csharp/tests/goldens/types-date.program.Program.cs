using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(TemperLang.Std.Temporal.TemporalGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(Date.DateGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
