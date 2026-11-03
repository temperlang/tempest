using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(TemperLang.Std.Testing.TestingGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(Asserts.AssertsGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
