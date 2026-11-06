package libraries;

import temper.core.Nullable;

public final class Held<T extends Shown> {
    public final T item;
    public final @Nullable Held<T> next;

    public Held(T item, @Nullable Held<T> next) {
        this.item = item;
        this.next = next;
    }

    public String toString() {
        return "Held(" + this.item.toString() + ")";
    }

    public T getItem() {
        return this.item;
    }

    public @Nullable Held<T> getNext() {
        return this.next;
    }

    public static final class Builder<T extends Shown> {
        T item;

        public Builder<T> item(T item) {
            this.item = item;
            return this;
        }

        @Nullable Held<T> next;
        boolean next__set;

        public Builder<T> next(@Nullable Held<T> next) {
            next__set = true;
            this.next = next;
            return this;
        }

        public Held<T> build() {
            if (item == null || !next__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (item == null) {
                    _message.append(" item");
                }
                if (!next__set) {
                    _message.append(" next");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Held<>(item, next);
        }
    }
}
