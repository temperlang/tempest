using TemperLang.Core;

namespace MutuallyReferencingTypes;

public static class MutuallyReferencingTypesGlobal
{
    static MutuallyReferencingTypesGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("MutuallyReferencingTypes");
        Foo emptyFoo = new Foo(null);
        Bar emptyBar = new Bar(null);
        Foo nonEmptyFoo = new Foo(emptyBar);
        Bar nonEmptyBar = new Bar(emptyFoo);
        Bar stuffedBar = new Bar(nonEmptyFoo);
        console.Log("emptyFoo.x has Foo -> " + (emptyFoo.X != null ? "true" : "false"));
        console.Log("nonEmptyFoo.x has Foo -> " + (nonEmptyFoo.X != null ? "true" : "false"));
    }
}
