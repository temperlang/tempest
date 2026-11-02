package build;

final class StringBuilderOwner {
    public final StringBuilder builder;

    public StringBuilderOwner(StringBuilder builder) {
        this.builder = builder;
    }

    public StringBuilder getBuilder() {
        return this.builder;
    }
}
