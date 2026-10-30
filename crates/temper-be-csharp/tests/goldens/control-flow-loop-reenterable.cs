using System.Globalization;
using TemperLang.Core;

namespace LoopReenterable;

public static class LoopReenterableGlobal
{
    static LoopReenterableGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("LoopReenterable");
        int i = 0;
        while (i < 2)
        {
            int x = i;
            console.Log(x.ToString(CultureInfo.InvariantCulture));
            i = i + 1;
        }
    }
}
