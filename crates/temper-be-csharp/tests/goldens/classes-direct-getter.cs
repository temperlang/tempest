using TemperLang.Core;

namespace DirectGetter;

public static class DirectGetterGlobal
{
    static DirectGetterGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("DirectGetter");
        C c = new C();
        console.Log("Hello, " + c.Place + "!");
    }
}
