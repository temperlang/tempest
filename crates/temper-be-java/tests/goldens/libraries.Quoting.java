package libraries;

import temper.core.Nullable;

public final class Quoting implements Escapes {
    public Quoting() {}

    public String escape(@Nullable String s) {
        return "'" + (s == null ? "null" : s) + "'";
    }
}
