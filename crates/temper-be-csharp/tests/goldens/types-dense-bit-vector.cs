using System.Collections;
using TemperLang.Core;

namespace DenseBitVector;

public static class DenseBitVectorGlobal
{
    static DenseBitVectorGlobal()
    {
        ILoggingConsole console = Support.Logging.LoggingConsoleFactory.CreateConsole("DenseBitVector");
        BitArray v = new BitArray(16);
        console.Log("v[2]=" + (Core.BitGet(v, 2) ? "true" : "false"));
        console.Log("v[3]=" + (Core.BitGet(v, 3) ? "true" : "false"));
        console.Log("v[3] <- true");
        Core.BitSet(v, 3, true);
        console.Log("v[2]=" + (Core.BitGet(v, 2) ? "true" : "false"));
        console.Log("v[3]=" + (Core.BitGet(v, 3) ? "true" : "false"));
        console.Log("v[3] <- false");
        Core.BitSet(v, 3, false);
        console.Log("v[2]=" + (Core.BitGet(v, 2) ? "true" : "false"));
        console.Log("v[3]=" + (Core.BitGet(v, 3) ? "true" : "false"));
    }
}
