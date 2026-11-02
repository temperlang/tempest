using System.Globalization;
using TemperLang.Core;

namespace Work;

public static class WorkGlobal
{
    internal static int SumOf3(int a, int b, int c)
    {
        return a + b + c;
    }

    public static int Sum(int i, int j, int? bonus = null)
    {
        int bonus2 = bonus ?? 0;
        return SumOf3(i, j, bonus2) + 1000;
    }

    public static int Length(string? @string = null)
    {
        if (@string == null)
        {
            return 999;
        }
        else
        {
            return StringUtil.CountBetween(@string, 0, @string.Length) + 1000;
        }
    }

    public static int Twice(int i)
    {
        return i * 2 + 1000;
    }

    internal static int Prod(Hidden hidden, int j)
    {
        return WorkConnected.Prod(hidden, j);
    }

    static WorkGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Work");
        console.Log("sum(1, 2): " + Sum(1, 2).ToString(CultureInfo.InvariantCulture));
        console.Log(string.Create(CultureInfo.InvariantCulture, $"length \"\" vs null vs (): {Length("")} vs {Length(null)} vs {Length()}"));
        console.Log("twice(5): " + Twice(5).ToString(CultureInfo.InvariantCulture));
        bool isConnected = Sum(1, 2) == 3;
        if (isConnected)
        {
            console.Log("prod(new Hidden(1), 2): " + Prod(new Hidden(1), 2).ToString(CultureInfo.InvariantCulture));
        }
        Hider hider = new Hider(3);
        console.Log("new Hider(3).plus(4): " + hider.Plus(4).ToString(CultureInfo.InvariantCulture));
        if (isConnected)
        {
            console.Log("new Hider(3).times(4): " + hider.Times(4).ToString(CultureInfo.InvariantCulture));
        }
    }
}
