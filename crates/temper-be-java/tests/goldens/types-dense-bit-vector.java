package dense_bit_vector;

import java.util.BitSet;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class DenseBitVectorGlobal {
    private DenseBitVectorGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("dense_bit_vector"));
        BitSet v = new BitSet(16);
        console.log("v[2]=" + v.get(2));
        console.log("v[3]=" + v.get(3));
        console.log("v[3] <- true");
        v.set(3, true);
        console.log("v[2]=" + v.get(2));
        console.log("v[3]=" + v.get(3));
        console.log("v[3] <- false");
        v.set(3, false);
        console.log("v[2]=" + v.get(2));
        console.log("v[3]=" + v.get(3));
    }
}
