package as_expr;

import java.util.Objects;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Nullable;

public final class AsExprGlobal {
    private AsExprGlobal() {}

    public static final Object thing0;
    public static final Object thing1;
    public static final Object thing2;
    public static final Object thing3;

    static {
        Console console = Core.getConsole(Logger.getLogger("as_expr"));
        {
            @Nullable Apple x;
            x = new Fuji();
            console.log(Objects.requireNonNull(x).getName());
            console.log(((Fuji) Objects.requireNonNull(x)).getName());
            x = new Gala();
            String t;
            try {
                t = ((Fuji) Objects.requireNonNull(x)).getName();
            } catch (RuntimeException ignored) {
                t = "Cast failed!";
            }
            console.log(t);
            @Nullable Object y = x;
            String t2;
            try {
                t2 = ((Apple) Objects.requireNonNull(y)).getName();
            } catch (RuntimeException ignored2) {
                t2 = "Cast from AnyValue failed!";
            }
            console.log(t2);
            String t3;
            try {
                t3 = ((Gala) Objects.requireNonNull(y)).getName();
            } catch (RuntimeException ignored3) {
                t3 = "Cast from AnyValue failed!";
            }
            console.log(t3);
            String t4;
            try {
                t4 = ((Fuji) Objects.requireNonNull(y)).getName();
            } catch (RuntimeException ignored4) {
                t4 = "Cast from AnyValue failed!";
            }
            console.log(t4);
            x = null;
            String t5;
            try {
                Objects.requireNonNull(x);
                t5 = "Tasty nothing.";
            } catch (RuntimeException ignored5) {
                t5 = "Was null!";
            }
            console.log(t5);
        }
        thing0 = false;
        thing1 = 1;
        thing2 = "two";
        thing3 = 3.14;
    }
}
