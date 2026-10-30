package types.exporter;

public final class ExportedClass {
    public final int foo;

    public ExportedClass(int foo) {
        this.foo = foo;
    }

    public int getFoo() {
        return this.foo;
    }
}
