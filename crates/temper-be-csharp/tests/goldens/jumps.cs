using System.Globalization;
using TemperLang.Core;

namespace Jumps;

public static class JumpsGlobal
{
    internal static int Search(int n)
    {
        int found = -1;
        int i = 0;
        while (i < n)
        {
            int j = 0;
            while (true)
            {
                if (j >= n)
                {
                    break;
                }
                if (i * j == 6)
                {
                    found = i * 10 + j;
                    goto outerEnd;
                }
                j = j + 1;
            }
            i = i + 1;
        }
        outerEnd: ;
        return found;
    }

    internal static int Odds(int n)
    {
        int k = 0;
        int sum = 0;
        do
        {
            k = k + 1;
            if ((k & 1) == 0)
            {
                continue;
            }
            sum = sum + k;
        } while (k < n);
        return sum;
    }

    static JumpsGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Jumps");
        console.Log(Search(5).ToString(CultureInfo.InvariantCulture));
        console.Log(Odds(9).ToString(CultureInfo.InvariantCulture));
    }
}
