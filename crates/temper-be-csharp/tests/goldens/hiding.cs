using System.Globalization;
using TemperLang.Core;

namespace Hiding;

public static class HidingGlobal
{
    public static readonly string Support;

    internal static int Math(int a, int b)
    {
        return global::System.Math.Min(a, b);
    }

    internal static int Twice(int @object)
    {
        return @object + @object;
    }

    internal static int Core(int a, int b)
    {
        return global::TemperLang.Core.Core.Div(a, b);
    }

    internal static bool Before(bool a, bool b)
    {
        return a.CompareTo(b) < 0;
    }

    static HidingGlobal()
    {
        ILoggingConsole console = global::Hiding.Support.Logging.LoggingConsoleFactory.CreateConsole("Hiding");
        Support = "s";
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{Math(3, 2)} {Twice(2)} {Core(7, 2)} {Support}"));
        console.Log(Before(false, true) ? "true" : "false");
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{(global::System.Math.Min(3, 2))} {(global::System.Math.Max(4, 1))}"));
    }
}
