using Modules;
using Modules.Helpers;
using Renamed;
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
        Checked @checked = new Checked(RenamedGlobal.Lambda(RenamedGlobal.Native));
        console.Log(string.Create(CultureInfo.InvariantCulture, $"renamed {@checked.Delete()} {@checked.Local} {@checked.End}"));
        Builtins b = new Builtins(4);
        console.Log(string.Create(CultureInfo.InvariantCulture, $"builtins {b.Hash} {b.Get(1)} {b.Set(3)} {b.Map(5)} {b.Format()} {b.Int} {b.Later()}"));
        console.Log(b.Print());
    }
}
