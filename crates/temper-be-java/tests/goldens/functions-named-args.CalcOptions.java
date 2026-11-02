package named_args;

import temper.core.Nullable;

final class CalcOptions {
    public final int scale;
    public final int origin;

    public CalcOptions(@Nullable Integer scale, @Nullable Integer origin) {
        int scale2 = scale != null ? scale : 1;
        int origin2 = origin != null ? origin : 0;
        this.scale = scale2;
        this.origin = origin2;
    }

    public CalcOptions(@Nullable Integer scale) {
        this(scale, null);
    }

    public CalcOptions() {
        this(null, null);
    }

    public int getScale() {
        return this.scale;
    }

    public int getOrigin() {
        return this.origin;
    }

    public static final class Builder {
        @Nullable Integer scale;

        public Builder scale(@Nullable Integer scale) {
            this.scale = scale;
            return this;
        }

        @Nullable Integer origin;

        public Builder origin(@Nullable Integer origin) {
            this.origin = origin;
            return this;
        }

        public CalcOptions build() {
            return new CalcOptions(scale, origin);
        }
    }
}
