package libraries;

import temper.core.Nullable;

public interface Escapes {
    String escape(@Nullable String s);

    default String escapeFlag(boolean b) {
        return this.escape(b ? "T" : "F");
    }
}
