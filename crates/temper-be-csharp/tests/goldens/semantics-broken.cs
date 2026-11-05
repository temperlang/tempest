using TemperLang.Core;

namespace Broken;

public static class BrokenGlobal
{
    internal static int UndefinedPlusArgInside()
    {
        return Core.Bubble<int>();
    }

    internal static string PadLeft(string @string, int minSize, string pad)
    {
        int needed = minSize - StringUtil.CountBetween(@string, 0, @string.Length);
        if (needed <= 0)
        {
            return @string;
        }
        return Core.Bubble<string>();
    }

    static BrokenGlobal()
    {
        Core.Bubble();
    }
}
