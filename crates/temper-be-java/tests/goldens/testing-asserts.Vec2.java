package asserts;

import temper.core.Core;

public final class Vec2 {
    public final double x;
    public final double y;

    public Vec2(double x, double y) {
        this.x = x;
        this.y = y;
    }

    public String toString() {
        return "(" + Core.float64ToString(this.x) + ", " + Core.float64ToString(this.y) + ") size squared " + Core.float64ToString(AssertsGlobal.normSquared(this));
    }

    public double getX() {
        return this.x;
    }

    public double getY() {
        return this.y;
    }

    public static final class Builder {
        double x;
        boolean x__set;

        public Builder x(double x) {
            x__set = true;
            this.x = x;
            return this;
        }

        double y;
        boolean y__set;

        public Builder y(double y) {
            y__set = true;
            this.y = y;
            return this;
        }

        public Vec2 build() {
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
            return new Vec2(x, y);
        }
    }
}
