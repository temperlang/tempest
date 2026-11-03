package object_literals.extra_class;

public final class Circle implements Stringable {
    public final int radius;

    public Circle(int radius) {
        this.radius = radius;
    }

    public String toString() {
        return "Circle of radius " + this.radius;
    }

    public int getRadius() {
        return this.radius;
    }
}
