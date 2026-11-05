using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(TemperLang.Std.Regex.RegexGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(Match.MatchGlobal).TypeHandle);
        global::TemperLang.Core.Async.WaitUntilSafeToExit();
    }
}
