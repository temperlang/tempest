using TemperLang.Core;

namespace HelloWorldObject;

public static class HelloWorldObjectGlobal
{
    internal static readonly ILoggingConsole console;

    static HelloWorldObjectGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("HelloWorldObject");
        C c = new C("Hello", "World");
        console.Log(c.X + ", " + c.Y + "!");
        c.Echo("Hello World, again!");
    }
}
