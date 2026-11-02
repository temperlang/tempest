using System.Globalization;
using TemperLang.Core;

namespace Work;

public static class WorkGlobal
{
    static WorkGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Work");
        Counter counter = new Counter();
        counter.Add(2);
        counter.Add(3);
        console.Log("counter.total(): " + counter.Total().ToString(CultureInfo.InvariantCulture));
        console.Log("new Doubler().twice(5): " + new Doubler().Twice(5).ToString(CultureInfo.InvariantCulture));
    }
}
