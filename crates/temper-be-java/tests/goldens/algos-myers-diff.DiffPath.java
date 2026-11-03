package myers_diff;

import temper.core.Nullable;

final class DiffPath {
    public final int leftIndex;
    public final int rightIndex;
    public final String changeType;
    public final @Nullable DiffPath previous;

    public DiffPath(int leftIndex, int rightIndex, String changeType, @Nullable DiffPath previous) {
        this.leftIndex = leftIndex;
        this.rightIndex = rightIndex;
        this.changeType = changeType;
        this.previous = previous;
    }

    public int getLeftIndex() {
        return this.leftIndex;
    }

    public int getRightIndex() {
        return this.rightIndex;
    }

    public String getChangeType() {
        return this.changeType;
    }

    public @Nullable DiffPath getPrevious() {
        return this.previous;
    }

    public static final class Builder {
        int leftIndex;
        boolean leftIndex__set;

        public Builder leftIndex(int leftIndex) {
            leftIndex__set = true;
            this.leftIndex = leftIndex;
            return this;
        }

        int rightIndex;
        boolean rightIndex__set;

        public Builder rightIndex(int rightIndex) {
            rightIndex__set = true;
            this.rightIndex = rightIndex;
            return this;
        }

        String changeType;

        public Builder changeType(String changeType) {
            this.changeType = changeType;
            return this;
        }

        @Nullable DiffPath previous;
        boolean previous__set;

        public Builder previous(@Nullable DiffPath previous) {
            previous__set = true;
            this.previous = previous;
            return this;
        }

        public DiffPath build() {
            if (!leftIndex__set || !rightIndex__set || changeType == null || !previous__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (!leftIndex__set) {
                    _message.append(" leftIndex");
                }
                if (!rightIndex__set) {
                    _message.append(" rightIndex");
                }
                if (changeType == null) {
                    _message.append(" changeType");
                }
                if (!previous__set) {
                    _message.append(" previous");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new DiffPath(leftIndex, rightIndex, changeType, previous);
        }
    }
}
