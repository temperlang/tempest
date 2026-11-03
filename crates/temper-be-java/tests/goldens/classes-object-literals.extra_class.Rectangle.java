package object_literals.extra_class;

public final class Rectangle {
    public final int width;
    public final int height;

    public Rectangle(int width, int height) {
        this.width = width;
        this.height = height;
    }

    public int getWidth() {
        return this.width;
    }

    public int getHeight() {
        return this.height;
    }

    public static final class Builder {
        int width;
        boolean width__set;

        public Builder width(int width) {
            width__set = true;
            this.width = width;
            return this;
        }

        int height;
        boolean height__set;

        public Builder height(int height) {
            height__set = true;
            this.height = height;
            return this;
        }

        public Rectangle build() {
            if (!width__set || !height__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (!width__set) {
                    _message.append(" width");
                }
                if (!height__set) {
                    _message.append(" height");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Rectangle(width, height);
        }
    }
}
