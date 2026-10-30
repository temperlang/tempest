using System.Globalization;
using TemperLang.Core;

namespace Testing;

public static class TestingGlobal
{
    internal static volatile int runs;
    internal static readonly int @base;

    public static int Twice(int n)
    {
        return n * 2;
    }

    static TestingGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Testing");
        runs = 0;
        @base = 10;
        console.Log("init " + Twice(1).ToString(CultureInfo.InvariantCulture));
    }
}
