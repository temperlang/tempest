using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(Modules.ModulesGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(Modules.Helpers.HelpersGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(ImportsLibrary.ImportsLibraryGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
