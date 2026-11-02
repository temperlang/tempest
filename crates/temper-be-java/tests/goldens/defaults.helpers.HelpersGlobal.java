package defaults.helpers;

import temper.core.Nullable;

public final class HelpersGlobal {
    private HelpersGlobal() {}

    public static String wrap(String text, @Nullable String open_, @Nullable String close) {
        String open_2 = open_ != null ? open_ : "[";
        String close2 = close != null ? close : open_2;
        return open_2 + text + close2;
    }

    public static String wrap(String text, @Nullable String open_) {
        return wrap(text, open_, null);
    }

    public static String wrap(String text) {
        return wrap(text, null, null);
    }
}
