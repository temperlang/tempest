using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using TemperLang.Core;

namespace MinimalRepro;

public static class MinimalReproGlobal
{
    static MinimalReproGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("MinimalRepro");
        IReadOnlyList<int> ls = Listed.CreateReadOnlyList<int>(0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
        console.Log("All  " + Listed.Join(ls, ", ", (int x) => x.ToString(CultureInfo.InvariantCulture)));
        console.Log("Even " + Listed.Join(Listed.ToReadOnlyList(Enumerable.Where(ls, (int x) => (x & 1) == 0)), ", ", (int x) => x.ToString(CultureInfo.InvariantCulture)));
        console.Log("Odd  " + Listed.Join(Listed.ToReadOnlyList(Enumerable.Where(ls, (int x) => (x & 1) != 0)), ", ", (int x) => x.ToString(CultureInfo.InvariantCulture)));
        console.Log("Neg  " + Listed.Join(Listed.ToReadOnlyList(Enumerable.Select(ls, (int x) => -x)), ", ", (int x) => x.ToString(CultureInfo.InvariantCulture)));
        IReadOnlyList<string> strs = Core.Split("1 ; 2 ; 3", " ; ");
        console.Log("Splt " + Listed.Join(strs, ", ", (string x) => x));
    }
}
