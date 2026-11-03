package object_literals;

import temper.core.Nullable;

final class Moreover {
    public final int blech;
    public final int bling;

    public Moreover(@Nullable Integer blech, @Nullable Integer bling) {
        int blech2 = blech != null ? blech : 0;
        int bling2 = bling != null ? bling : 0;
        this.blech = blech2;
        this.bling = bling2;
    }

    public Moreover(@Nullable Integer blech) {
        this(blech, null);
    }

    public Moreover() {
        this(null, null);
    }

    public String toString() {
        return this.blech + " " + this.bling;
    }

    public int getBlech() {
        return this.blech;
    }

    public int getBling() {
        return this.bling;
    }

    public static final class Builder {
        @Nullable Integer blech;

        public Builder blech(@Nullable Integer blech) {
            this.blech = blech;
            return this;
        }

        @Nullable Integer bling;

        public Builder bling(@Nullable Integer bling) {
            this.bling = bling;
            return this;
        }

        public Moreover build() {
            return new Moreover(blech, bling);
        }
    }
}
