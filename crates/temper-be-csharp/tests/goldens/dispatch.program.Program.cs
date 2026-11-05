using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(Dispatch.DispatchGlobal).TypeHandle);
        global::TemperLang.Core.Async.WaitUntilSafeToExit();
    }
}
