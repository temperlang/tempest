package libraries;

public final class Casing {
    public final String name;
    public static final Casing first = new Casing("first");

    public boolean eq(Casing other) {
        return this.name.equals(other.name);
    }

    Casing(String name) {
        this.name = name;
    }

    public String toString() {
        return this.name;
    }

    public String getName() {
        return this.name;
    }
}
