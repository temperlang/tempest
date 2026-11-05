using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(Locals.LocalsGlobal).TypeHandle);
        global::TemperLang.Core.Async.WaitUntilSafeToExit();
    }
}
