using System.Globalization;
using TemperLang.Core;

namespace WriteBackThroughField;

public static class WriteBackThroughFieldGlobal
{
    static WriteBackThroughFieldGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("WriteBackThroughField");
        Ping p = new Ping(new Pong());
        console.Log("ping=" + p.Ping_().ToString(CultureInfo.InvariantCulture));
        console.Log("pass=" + p.Pass().ToString(CultureInfo.InvariantCulture));
        console.Log("record=" + (p.Record() ? "true" : "false") + " most=" + p.Most.ToString(CultureInfo.InvariantCulture));
        console.Log(string.Create(CultureInfo.InvariantCulture, $"around={p.Around()} n={p.N}"));
    }
}
