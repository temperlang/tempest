package classes;

final class Point {
    public final int x;
    public final int y;

    public Point(int x, int y) {
        this.x = x;
        this.y = y;
    }

    public boolean eq(Point other) {
        if (this.x == other.x) {
            return this.y == other.y;
        } else {
            return false;
        }
    }

    public int getX() {
        return this.x;
    }

    public int getY() {
        return this.y;
    }

    public static final class Builder {
        int x;
        boolean x__set;

        public Builder x(int x) {
            x__set = true;
            this.x = x;
            return this;
        }

        int y;
        boolean y__set;

        public Builder y(int y) {
            y__set = true;
            this.y = y;
            return this;
        }

        public Point build() {
            if (!x__set || !y__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (!x__set) {
                    _message.append(" x");
                }
                if (!y__set) {
                    _message.append(" y");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new Point(x, y);
        }
    }
}
