using TemperLang.Core;

namespace HelloWorld;

public static class HelloWorldGlobal
{
    static HelloWorldGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("HelloWorld");
        console.Log("Hello, World!");
    }
}
