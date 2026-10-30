using Modules;
using Modules.Helpers;
using System.Globalization;
using TemperLang.Core;

namespace ImportsLibrary;

public static class ImportsLibraryGlobal
{
    public static int Both(int n)
    {
        return ModulesGlobal.Sum(n) + HelpersGlobal.Twice(n) + HelpersGlobal.Base;
    }

    static ImportsLibraryGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("ImportsLibrary");
        console.Log("both " + Both(2).ToString(CultureInfo.InvariantCulture));
    }
}
