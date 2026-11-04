using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(TemperLang.Std.Json.JsonGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(JsonSyntaxTree.JsonSyntaxTreeGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
