using System.Globalization;
using TemperLang.Core;

namespace PropertyOrder;

public static class PropertyOrderGlobal
{
    internal static readonly ILoggingConsole console;

    static PropertyOrderGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("PropertyOrder");
        console.Log("about to create instance");
        C c = new C();
        console.Log("about to get backed");
        console.Log("c.backed=" + c.Backed.ToString(CultureInfo.InvariantCulture));
        console.Log("about to get computed");
        console.Log("c.computed=" + c.Computed.ToString(CultureInfo.InvariantCulture));
        console.Log("done");
    }
}
