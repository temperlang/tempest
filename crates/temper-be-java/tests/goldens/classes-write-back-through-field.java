package write_back_through_field;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class WriteBackThroughFieldGlobal {
    private WriteBackThroughFieldGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("write_back_through_field"));
        Ping p = new Ping(new Pong());
        console.log("ping=" + p.ping());
        console.log("pass=" + p.pass());
        console.log("record=" + p.record_() + " most=" + p.most);
        console.log("around=" + p.around() + " n=" + p.n);
    }
}
