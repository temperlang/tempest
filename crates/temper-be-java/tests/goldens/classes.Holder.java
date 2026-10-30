package classes;

final class Holder {
    public final Loose loose;
    public final String tag;

    public Holder(Loose loose, String tag) {
        this.loose = loose;
        this.tag = tag;
    }

    public boolean eq(Holder other) {
        if (this.loose.eq(other.loose)) {
            return this.tag.equals(other.tag);
        } else {
            return false;
        }
    }

    public Loose getLoose() {
        return this.loose;
    }

    public String getTag() {
        return this.tag;
    }

    public static final class Builder {
        Loose loose;

        public Builder loose(Loose loose) {
            this.loose = loose;
            return this;
        }

        String tag;

        public Builder tag(String tag) {
            this.tag = tag;
            return this;
        }

        public Holder build() {
            if (loose == null || tag == null) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (loose == null) {
                    _message.append(" loose");
                }
                if (tag == null) {
                    _message.append(" tag");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Holder(loose, tag);
        }
    }
}
