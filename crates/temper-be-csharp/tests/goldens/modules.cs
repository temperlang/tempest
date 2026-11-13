using Modules.Core;
using Modules.Helpers;
using Modules.Helpers.Deep;
using Modules.Machine;
using System.Globalization;
using TemperLang.Core;

namespace Modules;

public static class ModulesGlobal
{
    internal static readonly int @base;

    public static int Sum(int n)
    {
        return HelpersGlobal.Twice(n) + HelpersGlobal.Base + @base;
    }

    static ModulesGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Modules");
        int calls = HelpersGlobal.Calls;
        @base = 1;
        console.Log(DeepGlobal.Greet("modules"));
        console.Log("sum " + Sum(3).ToString(CultureInfo.InvariantCulture));
        console.Log(string.Create(CultureInfo.InvariantCulture, $"share {CoreGlobal.Share(7, 2)} {HelpersGlobal.IntDoubled(21)}"));
        console.Log(new Engine("own").Name + ", " + MachineGlobal.Describe());
        HelpersGlobal.Bump();
        HelpersGlobal.Bump();
        console.Log("calls " + calls.ToString(CultureInfo.InvariantCulture));
        console.Log("calls now " + HelpersGlobal.Bump().ToString(CultureInfo.InvariantCulture));
    }
}
