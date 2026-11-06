using System.Globalization;
using TemperLang.Core;

namespace Triple;

public static class TripleGlobal
{
    internal static int Triple(int n)
    {
        return n * 3;
    }

    static TripleGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Triple");
        console.Log("triple(14) = " + Triple(14).ToString(CultureInfo.InvariantCulture));
    }
}
