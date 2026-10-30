using TemperLang.Core;

namespace InheritedGetter;

public static class InheritedGetterGlobal
{
    static InheritedGetterGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("InheritedGetter");
        C c = new C();
        console.Log("Goodbye, gotta run, " + ((II)c).Place + "!");
    }
}
