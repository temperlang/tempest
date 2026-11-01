using System;
using System.Globalization;
using TemperLang.Core;

namespace SimpleLocals;

public static class SimpleLocalsGlobal
{
    internal static readonly ILoggingConsole console;

    internal static int CallIt(int i, Func<int, int> f)
    {
        return 3 * f(i);
    }

    internal static void Enclose(int i)
    {
        int result = CallIt(2, (int j) => i + j);
        console.Log(result.ToString(CultureInfo.InvariantCulture));
    }

    static SimpleLocalsGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("SimpleLocals");
        Enclose(1);
    }
}
