package asserts;

final class TestHelper {
    public final int i;
    public final int j;

    public TestHelper(int i, int j) {
        this.i = i;
        this.j = j;
    }

    public int getI() {
        return this.i;
    }

    public int getJ() {
        return this.j;
    }

    public static final class Builder {
        int i;
        boolean i__set;

        public Builder i(int i) {
            i__set = true;
            this.i = i;
            return this;
        }

        int j;
        boolean j__set;

        public Builder j(int j) {
            j__set = true;
            this.j = j;
            return this;
        }

        public TestHelper build() {
            if (!i__set || !j__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (!i__set) {
                    _message.append(" i");
                }
                if (!j__set) {
                    _message.append(" j");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new TestHelper(i, j);
        }
    }
}
