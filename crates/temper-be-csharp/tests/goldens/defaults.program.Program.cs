using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(Defaults.Helpers.HelpersGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(Defaults.DefaultsGlobal).TypeHandle);
        global::TemperLang.Core.Async.WaitUntilSafeToExit();
    }
}
