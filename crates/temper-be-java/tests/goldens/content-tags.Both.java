package content_tags;

public final class Both implements Escaper {
    public final Escaper first;
    public final Escaper second;

    public Both(Escaper first, Escaper second) {
        this.first = first;
        this.second = second;
    }

    public String applyString(String x) {
        return this.second.applyString(this.first.applyString(x));
    }

    public Escaper getFirst() {
        return this.first;
    }

    public Escaper getSecond() {
        return this.second;
    }

    public static final class Builder {
        Escaper first;

        public Builder first(Escaper first) {
            this.first = first;
            return this;
        }

        Escaper second;

        public Builder second(Escaper second) {
            this.second = second;
            return this;
        }

        public Both build() {
            if (first == null || second == null) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (first == null) {
                    _message.append(" first");
                }
                if (second == null) {
                    _message.append(" second");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Both(first, second);
        }
    }
}
