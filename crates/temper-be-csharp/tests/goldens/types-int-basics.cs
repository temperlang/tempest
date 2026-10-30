using System;
using System.Globalization;
using TemperLang.Core;

namespace Basics;

public static class BasicsGlobal
{
    static BasicsGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Basics");
        console.Log("1.min(3)");
        console.Log(Math.Min(1, 3).ToString(CultureInfo.InvariantCulture));
        console.Log("3.min(1)");
        console.Log(Math.Min(3, 1).ToString(CultureInfo.InvariantCulture));
        console.Log("1.max(3)");
        console.Log(Math.Max(1, 3).ToString(CultureInfo.InvariantCulture));
        console.Log("3.max(1)");
        console.Log(Math.Max(3, 1).ToString(CultureInfo.InvariantCulture));
    }
}
