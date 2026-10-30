package hello_world_object;

final class C {
    public final String x;
    public final String y;

    public C(String x, String y) {
        this.x = x;
        this.y = y;
    }

    public void echo(String input) {
        HelloWorldObjectGlobal.console.log(input);
    }

    public String getX() {
        return this.x;
    }

    public String getY() {
        return this.y;
    }

    public static final class Builder {
        String x;

        public Builder x(String x) {
            this.x = x;
            return this;
        }

        String y;

        public Builder y(String y) {
            this.y = y;
            return this;
        }

        public C build() {
            if (x == null || y == null) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (x == null) {
                    _message.append(" x");
                }
                if (y == null) {
                    _message.append(" y");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new C(x, y);
        }
    }
}
