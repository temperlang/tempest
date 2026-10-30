using TemperLang.Core;

namespace IsEmpty;

public static class IsEmptyGlobal
{
    static IsEmptyGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("IsEmpty");
        string emptyString = "";
        string foo = "foo";
        console.Log(emptyString.Length == 0 ? "emptyString is empty" : "emptyString is not empty");
        console.Log(foo.Length == 0 ? "foo is empty" : "foo is not empty");
    }
}
