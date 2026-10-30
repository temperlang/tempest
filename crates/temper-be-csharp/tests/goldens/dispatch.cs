using TemperLang.Core;

namespace Dispatch;

public static class DispatchGlobal
{
    internal static string Pick(bool a, bool b)
    {
        if (a)
        {
            if (b)
            {
                return "ab";
            }
        }
        else
        {
            return "not a";
        }
        return "a only";
    }

    static DispatchGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Dispatch");
        IA v = new X();
        console.Log(v.A() + " " + v.B);
        console.Log(Pick(true, true) + " " + Pick(true, false) + " " + Pick(false, true));
    }
}
