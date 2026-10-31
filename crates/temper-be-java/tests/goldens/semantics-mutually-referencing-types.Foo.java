package mutually_referencing_types;

import temper.core.Nullable;

final class Foo {
    public final @Nullable Bar x;

    public Foo(@Nullable Bar x) {
        this.x = x;
    }

    public @Nullable Bar getX() {
        return this.x;
    }
}
