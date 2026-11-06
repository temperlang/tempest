package libraries;

public final class Casing {
    public final String name;
    public static final Casing first = new Casing("first");
    public static final Casing last = named("last");

    public boolean eq(Casing other) {
        return this.name.equals(other.name);
    }

    Casing(String name) {
        this.name = name;
    }

    public String toString() {
        return this.name;
    }

    static Casing named(String name) {
        return new Casing(name);
    }

    public String getName() {
        return this.name;
    }
}
