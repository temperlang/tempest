package libraries;

import java.util.List;

public final class Bundle<T> {
    public final List<Effect<T>> effects;
    public final T first;

    public Bundle(List<Effect<T>> effects, T first) {
        this.effects = effects;
        this.first = first;
    }

    public List<Effect<T>> getEffects() {
        return this.effects;
    }

    public T getFirst() {
        return this.first;
    }

    public static final class Builder<T> {
        List<Effect<T>> effects;

        public Builder<T> effects(List<Effect<T>> effects) {
            this.effects = effects;
            return this;
        }

        T first;

        public Builder<T> first(T first) {
            this.first = first;
            return this;
        }

        public Bundle<T> build() {
            if (effects == null || first == null) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (effects == null) {
                    _message.append(" effects");
                }
                if (first == null) {
                    _message.append(" first");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Bundle<>(effects, first);
        }
    }
}
