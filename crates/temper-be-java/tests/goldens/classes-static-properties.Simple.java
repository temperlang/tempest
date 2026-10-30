package static_properties;

final class Simple {
    public final String name;
    public static final Simple simon = new Simple("foo");
    public static final String value = Simple.simon.name;
    public static final int other = 22;

    public Simple(String name) {
        this.name = name;
    }

    public String getName() {
        return this.name;
    }
}
