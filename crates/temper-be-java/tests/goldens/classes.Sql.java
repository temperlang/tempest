package classes;

import java.util.ArrayList;
import java.util.List;
import temper.core.Core;
import temper.core.Nullable;

final class Sql {
    private final List<String> parts;

    public Sql() {
        this.parts = new ArrayList<>();
    }

    public void appendSafe(String s) {
        this.parts.add(s);
    }

    public void appendString(@Nullable String x) {
        this.parts.add(x == null ? "NULL" : "'" + x + "'");
    }

    public void appendInt(int x) {
        this.parts.add(Integer.toString(x));
    }

    public void appendShape(Shape x) {
        this.parts.add("shape " + x.getArea());
    }

    public void appendRect(Rect x) {
        this.parts.add("rect " + x.toString());
    }

    public String getAccumulated() {
        return Core.listJoinObj(this.parts, "", p -> p);
    }
}
