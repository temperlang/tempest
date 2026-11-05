package classes;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class ClassesGlobal {
    private ClassesGlobal() {}

    static Shape shape(Shape x) {
        return x;
    }

    static Class<?> typeNamed(int n) {
        if (n == 0) {
            return Void.class;
        } else if (n == 1) {
            return Boolean.class;
        } else if (n == 2) {
            return Integer.class;
        } else if (n == 3) {
            return Long.class;
        } else if (n == 4) {
            return Double.class;
        } else {
            return String.class;
        }
    }

    static Class<?> same(Class<?> t) {
        return t;
    }

    static {
        Console console = Core.getConsole(Logger.getLogger("classes"));
        Rect r = new Rect(2, 3);
        console.log(r.toString());
        console.log(Integer.toString(r.getArea()));
        Shape s = shape(r);
        console.log(s.describe());
        Shape q = shape(new Square(4));
        console.log(q.describe());
        console.log(Integer.toString(q.getArea()));
        console.log(Integer.toString(r.scaled(2).scaled(3).getArea()));
        console.log(Rect.unit.scaled(5).toString());
        Counter c = Counter.zero();
        c.bump().bump();
        console.log(Integer.toString(c.count));
        c.setDoubled(10);
        console.log(Integer.toString(c.getDoubled()));
        console.log(Boolean.toString(c.same(new Counter().bump().bump())));
        console.log(new Point(1, 2).eq(new Point(1, 2)) + " " + !new Point(1, 2).eq(new Point(2, 1)));
        console.log(Boolean.toString(new Holder(new Loose(3), "a").eq(new Holder(new Loose(-3), "a"))));
        console.log((new Version(1, 2).compareTo(new Version(1, 3)) < 0) + " " + (new Version(2, 0).compareTo(new Version(1, 9)) >= 0));
        console.log((q instanceof Square) + " " + (s instanceof Square));
        Square sq = (Square) q;
        console.log(Integer.toString(sq.getArea()));
        if (s instanceof Rect) {
            console.log(Integer.toString(((Rect) s).w));
        }
        Class<?> favorite = same(Long.class);
        int named = 0;
        {
            int i = 0;
            while (i < 6) {
                typeNamed(i);
                named = named + 1;
                i = i + 1;
            }
        }
        console.log(named + " types");
    }
}
