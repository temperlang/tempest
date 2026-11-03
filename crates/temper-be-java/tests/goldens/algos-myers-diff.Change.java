package myers_diff;

import java.util.List;

public final class Change<T> {
    public final String type;
    public final int leftIndex;
    public final int rightIndex;
    public final List<T> items;

    public Change(String type, int leftIndex, int rightIndex, List<T> items) {
        this.type = type;
        this.leftIndex = leftIndex;
        this.rightIndex = rightIndex;
        this.items = items;
    }

    public String getType() {
        return this.type;
    }

    public int getLeftIndex() {
        return this.leftIndex;
    }

    public int getRightIndex() {
        return this.rightIndex;
    }

    public List<T> getItems() {
        return this.items;
    }

    public static final class Builder<T> {
        String type;

        public Builder<T> type(String type) {
            this.type = type;
            return this;
        }

        int leftIndex;
        boolean leftIndex__set;

        public Builder<T> leftIndex(int leftIndex) {
            leftIndex__set = true;
            this.leftIndex = leftIndex;
            return this;
        }

        int rightIndex;
        boolean rightIndex__set;

        public Builder<T> rightIndex(int rightIndex) {
            rightIndex__set = true;
            this.rightIndex = rightIndex;
            return this;
        }

        List<T> items;

        public Builder<T> items(List<T> items) {
            this.items = items;
            return this;
        }

        public Change<T> build() {
            if (type == null || !leftIndex__set || !rightIndex__set || items == null) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (type == null) {
                    _message.append(" type");
                }
                if (!leftIndex__set) {
                    _message.append(" leftIndex");
                }
                if (!rightIndex__set) {
                    _message.append(" rightIndex");
                }
                if (items == null) {
                    _message.append(" items");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Change<>(type, leftIndex, rightIndex, items);
        }
    }
}
