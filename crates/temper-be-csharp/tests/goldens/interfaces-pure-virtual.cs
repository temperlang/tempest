using System.Globalization;
using TemperLang.Core;

namespace PureVirtual;

public static class PureVirtualGlobal
{
    internal static readonly ILoggingConsole console;

    static PureVirtualGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("PureVirtual");
        IGreeter greeter = new HiGreeter();
        greeter.Greet();
        IWrapper<Box> boxWrapper = new BoxWrapper();
        IWrapper<int> intWrapper = new IntWrapper();
        console.Log(boxWrapper.Wrap(new Box("box")).Label);
        console.Log(intWrapper.Wrap(41).ToString(CultureInfo.InvariantCulture));
    }
}
