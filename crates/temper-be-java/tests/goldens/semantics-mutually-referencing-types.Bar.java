package mutually_referencing_types;

import temper.core.Nullable;

final class Bar {
    public final @Nullable Foo x;

    public Bar(@Nullable Foo x) {
        this.x = x;
    }

    public @Nullable Foo getX() {
        return this.x;
    }
}
