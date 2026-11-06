package libraries;

public final class Tag implements Shown {
    public final String name;

    public Tag(String name) {
        this.name = name;
    }

    public String toString() {
        return "<" + this.name + ">";
    }

    public String getName() {
        return this.name;
    }
}
