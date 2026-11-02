package defaults.helpers;

import temper.core.Nullable;

public final class Pad {
    public final int left;
    public final int right;

    public Pad(@Nullable Integer left, @Nullable Integer right) {
        int left2 = left != null ? left : 1;
        int right2 = right != null ? right : 2;
        this.left = left2;
        this.right = right2;
    }

    public Pad(@Nullable Integer left) {
        this(left, null);
    }

    public Pad() {
        this(null, null);
    }

    public String apply(String text, @Nullable String fill) {
        String fill2 = fill != null ? fill : ".";
        String out = text;
        {
            int i = 0;
            while (i < this.left) {
                out = fill2 + out;
                i = i + 1;
            }
        }
        {
            int i = 0;
            while (i < this.right) {
                out = out + fill2;
                i = i + 1;
            }
        }
        return out;
    }

    public String apply(String text) {
        return apply(text, null);
    }

    public int getLeft() {
        return this.left;
    }

    public int getRight() {
        return this.right;
    }

    public static final class Builder {
        @Nullable Integer left;

        public Builder left(@Nullable Integer left) {
            this.left = left;
            return this;
        }

        @Nullable Integer right;

        public Builder right(@Nullable Integer right) {
            this.right = right;
            return this;
        }

        public Pad build() {
            return new Pad(left, right);
        }
    }
}
