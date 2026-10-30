using System.Globalization;
using TemperLang.Core;

namespace StringOrder;

public static class StringOrderGlobal
{
    internal static string Show(string a, string b)
    {
        return StringUtil.CompareStringsByCodePoint(a, b).ToString(CultureInfo.InvariantCulture) + " " + (StringUtil.CompareStringsByCodePoint(a, b) < 0 ? "true" : "false") + " " + (StringUtil.CompareStringsByCodePoint(a, b) <= 0 ? "true" : "false") + " " + (StringUtil.CompareStringsByCodePoint(a, b) > 0 ? "true" : "false") + " " + (StringUtil.CompareStringsByCodePoint(a, b) >= 0 ? "true" : "false");
    }

    internal static int Cmp(double a, double b)
    {
        return Float64.Compare(a, b);
    }

    static StringOrderGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("StringOrder");
        console.Log(Show("｡", "😀"));
        console.Log(Show("😀", "｡"));
        console.Log(Show("\uE000", "𐀀"));
        console.Log(Show("\uFFFF", "\U0010FFFF"));
        console.Log(Show("a｡b", "a😀"));
        console.Log(Show("😀", "😁"));
        console.Log(Show("😀", "😀"));
        console.Log(Show("😀", "😀a"));
        console.Log(Show("", "😀"));
        console.Log(Show("", ""));
        console.Log(Show("z", "a"));
        console.Log(Show("abc", "abd"));
        console.Log(string.Create(CultureInfo.InvariantCulture, $"{Cmp(2.5, 1.0)} {Cmp(1.0, 2.5)} {Cmp(1.5, 1.5)}"));
    }
}
