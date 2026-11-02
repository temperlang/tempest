using Defaults.Helpers;
using System.Globalization;
using TemperLang.Core;

namespace Defaults;

public static class DefaultsGlobal
{
    internal static volatile int calls;
    private static readonly object moduleLock = new();

    internal static int Next()
    {
        lock (moduleLock)
        {
            calls = calls + 1;
        }
        return calls;
    }

    internal static string Range(int start, int? end = null, int? step = null)
    {
        int end2 = end ?? start + 10;
        int step2 = step ?? Next();
        return string.Create(CultureInfo.InvariantCulture, $"{start}..{end2} by {step2}");
    }

    internal static string Tag(string text, string? mark = null)
    {
        if (mark == null)
        {
            return text;
        }
        else
        {
            return mark + text;
        }
    }

    internal static string Named(string? name = null)
    {
        string? name2 = name ?? "nobody";
        {
            if (name2 == null)
            {
                return "null";
            }
            else
            {
                return name2;
            }
        }
    }

    static DefaultsGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Defaults");
        calls = 0;
        console.Log(Range(1));
        console.Log(Range(1, 5));
        console.Log(Range(1, 5, 3));
        console.Log(Range(2, null, 7));
        console.Log("calls " + calls.ToString(CultureInfo.InvariantCulture));
        console.Log(Tag("plain"));
        console.Log(Tag("marked", "#"));
        console.Log(Named());
        console.Log(Named(null));
        console.Log(Named("Ada"));
        console.Log(new Greeter().Greet());
        console.Log(new Greeter("Hi").Greet("you", 2));
        console.Log(new Greeter(null, "?").Greet(null, 2));
        console.Log(Greeter.Shout("hey"));
        Counter counter = new Counter();
        counter.Add();
        console.Log(string.Create(CultureInfo.InvariantCulture, $"count {counter.Add(5)} {new Counter(1).Add()}"));
        console.Log(HelpersGlobal.Wrap("a"));
        console.Log(HelpersGlobal.Wrap("b", "<"));
        console.Log(HelpersGlobal.Wrap("c", "<", ">"));
        Pad pad = new Pad();
        console.Log(pad.Apply("x"));
        console.Log(new Pad(0, 3).Apply("y", "-"));
    }
}
