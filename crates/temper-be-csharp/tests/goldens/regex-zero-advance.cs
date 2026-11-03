using TemperLang.Core;
using TemperLang.Std.Regex;

namespace ZeroAdvance;

public static class ZeroAdvanceGlobal
{
    internal static readonly Regex commaItemStartRegex;

    public static string BulletizeList(string text)
    {
        string replaced = commaItemStartRegex.Replace(text, (Match match) => "\n- ");
        return StringUtil.Slice(replaced, StringUtil.Next(replaced, 0), replaced.Length);
    }

    static ZeroAdvanceGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("ZeroAdvance");
        commaItemStartRegex = ((IRegexNode)new Sequence(Listed.CreateReadOnlyList<IRegexNode>(new Or(Listed.CreateReadOnlyList<IRegexNode>(RegexGlobal.Begin, new CodePoints(","))), new Repeat(RegexGlobal.Space, 0, null, false)))).Compiled();
        console.Log(BulletizeList("apples, bananas, cherries"));
    }
}
