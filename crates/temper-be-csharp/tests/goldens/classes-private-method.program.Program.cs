using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(PrivateMethod.PrivateMethodGlobal).TypeHandle);
        global::TemperLang.Core.Async.WaitUntilSafeToExit();
    }
}
