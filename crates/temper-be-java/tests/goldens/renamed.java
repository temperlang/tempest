package renamed;

public final class RenamedGlobal {
    private RenamedGlobal() {}

    public static final int native_;

    public static int lambda(int n) {
        return n + 1;
    }

    static {
        native_ = 2;
    }
}
