using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(Classes.ClassesGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
