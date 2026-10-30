using System.Globalization;
using TemperLang.Core;

namespace Setters;

public static class SettersGlobal
{
    static SettersGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Setters");
        console.Log(new Foo("initial").Bar);
        Foo foo = new Foo("initial");
        foo.Bar = "next";
        console.Log(foo.Bar);
        Bar bar = new Bar("initial");
        bar.UsingSet("next");
        console.Log(bar.Bar_);
        IWhatever what = new ConcreteWhatever(4);
        what.Something = 5;
        console.Log("something set to " + what.Something.ToString(CultureInfo.InvariantCulture));
    }
}
