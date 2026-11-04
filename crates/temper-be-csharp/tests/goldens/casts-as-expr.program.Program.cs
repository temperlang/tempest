using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(AsExpr.AsExprGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
