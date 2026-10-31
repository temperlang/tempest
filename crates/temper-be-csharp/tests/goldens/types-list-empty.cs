using System.Collections.Generic;
using TemperLang.Core;

namespace Empty;

public static class EmptyGlobal
{
    internal static readonly ILoggingConsole console;

    internal static void F(IReadOnlyList<int> b)
    {
        if (b.Count == 0)
        {
            console.Log("empty");
        }
        else
        {
            console.Log("not empty");
        }
    }

    static EmptyGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("Empty");
        F(Listed.CreateReadOnlyList<int>(2, 3));
        F(Listed.CreateReadOnlyList<int>());
    }
}
