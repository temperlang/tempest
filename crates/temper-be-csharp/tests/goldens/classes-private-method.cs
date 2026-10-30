using System.Globalization;
using TemperLang.Core;

namespace PrivateMethod;

public static class PrivateMethodGlobal
{
    static PrivateMethodGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("PrivateMethod");
        C c = new C();
        console.Log("method1(10)=" + c.Method1(10).ToString(CultureInfo.InvariantCulture));
        console.Log("method2(10)=" + c.Method2(10).ToString(CultureInfo.InvariantCulture));
    }
}
