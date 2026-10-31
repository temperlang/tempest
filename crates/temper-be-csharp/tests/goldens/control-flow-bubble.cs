using System.Globalization;
using TemperLang.Core;

namespace Bubble;

public static class BubbleGlobal
{
    internal static readonly ILoggingConsole console;

    internal static int More(int? x)
    {
        return Core.Bubble<int>();
    }

    internal static string Something(string? x)
    {
        {
            if (x != null)
            {
                return x;
            }
            else
            {
                return Core.Bubble<string>();
            }
        }
    }

    internal static void Report(string? x)
    {
        string message;
        try
        {
            message = Something(x);
        }
        catch
        {
            message = "null";
        }
        string suffix;
        try
        {
            int t1;
            try
            {
                Core.CastToNonNull<string>(x);
                t1 = 1;
            }
            catch
            {
                t1 = 0;
            }
            suffix = More(t1).ToString(CultureInfo.InvariantCulture);
        }
        catch
        {
            suffix = "";
        }
        console.Log(message + suffix);
    }

    internal static int CalcRows(int cols, int total)
    {
        console.Log(string.Create(CultureInfo.InvariantCulture, $"calcRows({cols}, {total})"));
        if (Core.Mod(total, cols) != 0)
        {
            return Core.Bubble<int>();
        }
        return Core.Div(total, cols);
    }

    internal static int CountBelow(string s)
    {
        int i = 0;
        while (i < Core.ToInt(s))
        {
            i = i + 1;
        }
        return i;
    }

    internal static void Branch(int a, int b)
    {
        string c = a == 0 ? (b == 0 ? "00" : b == 1 ? "01" : b == 2 ? "02" : "0x") : a == 1 ? "1x" : a == 2 ? "2x" : a == 3 ? "3x" : "xx";
        console.Log(string.Create(CultureInfo.InvariantCulture, $"branch({a}, {b}) -> {c}"));
    }

    static BubbleGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("Bubble");
        Report("hi");
        Report(null);
        console.Log(CalcRows(3, 6).ToString(CultureInfo.InvariantCulture));
        int t2;
        try
        {
            t2 = CountBelow("abc");
        }
        catch
        {
            t2 = -1;
        }
        console.Log(t2.ToString(CultureInfo.InvariantCulture));
        Branch(0, 2);
        Branch(2, 0);
    }
}
