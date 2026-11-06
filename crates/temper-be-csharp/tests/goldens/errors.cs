using System.Globalization;
using TemperLang.Core;

namespace Errors;

public static class ErrorsGlobal
{
    public static int Good(int x)
    {
        return x + 1;
    }

    internal static int Mismatched(int x)
    {
        return Core.Bubble<int>();
    }

    internal static int Undeclared()
    {
        return Core.Bubble<int>();
    }

    internal static int Misspelled(string s)
    {
        return Core.Bubble<int>();
    }

    internal static int MisspelledMethod(string s)
    {
        return Core.Bubble<int>();
    }

    internal static int WrongArity()
    {
        return Core.Bubble<int>();
    }

    internal static int UnknownType(object x)
    {
        return 1;
    }

    internal static int Unclosed(int x)
    {
        return x * 2;
    }

    internal static int AfterUnclosed(int x)
    {
        return x * 3;
    }

    static ErrorsGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Errors");
        int a;
        try
        {
            a = Mismatched(1);
        }
        catch
        {
            a = -1;
        }
        int b;
        try
        {
            b = Undeclared();
        }
        catch
        {
            b = -1;
        }
        int c;
        try
        {
            c = Misspelled("a");
        }
        catch
        {
            c = -1;
        }
        int d;
        try
        {
            d = MisspelledMethod("a");
        }
        catch
        {
            d = -1;
        }
        int e;
        try
        {
            e = WrongArity();
        }
        catch
        {
            e = -1;
        }
        Box box = new Box(3);
        string f;
        try
        {
            f = box.Bad();
        }
        catch
        {
            f = "caught";
        }
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{Good(1)} {a} {b} {c} {d} {e}"));
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{UnknownType(1)} {AfterUnclosed(2)} {box.Get()} {f}"));
    }
}
