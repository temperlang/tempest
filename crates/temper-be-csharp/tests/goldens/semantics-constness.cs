using System.Globalization;
using TemperLang.Core;

namespace Constness;

public static class ConstnessGlobal
{
    static ConstnessGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("Constness");
        {
            int nci = 1;
            console.Log("nci = " + nci.ToString(CultureInfo.InvariantCulture));
            nci = 2;
            console.Log("nci = " + nci.ToString(CultureInfo.InvariantCulture));
            int nc;
            nc = 1;
            console.Log("nc = " + nc.ToString(CultureInfo.InvariantCulture));
            nc = 2;
            console.Log("nc = " + nc.ToString(CultureInfo.InvariantCulture));
            {
                int i = 0;
                while (i < 3)
                {
                    int ncl;
                    ncl = i;
                    console.Log("ncl = " + ncl.ToString(CultureInfo.InvariantCulture));
                    i = i + 1;
                }
            }
            int ci = 1;
            console.Log("ci = " + ci.ToString(CultureInfo.InvariantCulture));
            console.Log("failed to set ci");
            console.Log("ci = " + ci.ToString(CultureInfo.InvariantCulture));
            int c;
            c = 1;
            console.Log("c = " + c.ToString(CultureInfo.InvariantCulture));
            console.Log("failed to set c");
            console.Log("c = " + c.ToString(CultureInfo.InvariantCulture));
            {
                int i2 = 0;
                while (i2 < 3)
                {
                    int cl;
                    cl = i2;
                    console.Log("cl = " + cl.ToString(CultureInfo.InvariantCulture));
                    i2 = i2 + 1;
                }
            }
        }
    }
}
