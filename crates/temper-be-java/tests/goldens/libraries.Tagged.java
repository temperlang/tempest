package libraries;

import temper.core.Nullable;

public final class Tagged {
    public final @Nullable String label;

    public Tagged(@Nullable String label) {
        this.label = label;
    }

    public @Nullable String getLabel() {
        return this.label;
    }
}
