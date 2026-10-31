package nullables;

import temper.core.Nullable;

public final class Node {
    public final int value;
    public final @Nullable Node next;

    public Node(int value, @Nullable Node next) {
        this.value = value;
        this.next = next;
    }

    public int last() {
        @Nullable Node n = this.next;
        if (n == null) {
            return this.value;
        } else {
            return n.last();
        }
    }

    public int getValue() {
        return this.value;
    }

    public @Nullable Node getNext() {
        return this.next;
    }

    public static final class Builder {
        int value;
        boolean value__set;

        public Builder value(int value) {
            value__set = true;
            this.value = value;
            return this;
        }

        @Nullable Node next;
        boolean next__set;

        public Builder next(@Nullable Node next) {
            next__set = true;
            this.next = next;
            return this;
        }

        public Node build() {
            if (!value__set || !next__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (!value__set) {
                    _message.append(" value");
                }
                if (!next__set) {
                    _message.append(" next");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Node(value, next);
        }
    }
}
