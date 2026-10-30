package classes;

public final class Square implements Shape {
    private final int side;

    public Square(int side) {
        this.side = side;
    }

    public int getArea() {
        return this.side * this.side;
    }

    public String describe() {
        return "square of side " + this.side;
    }
}
