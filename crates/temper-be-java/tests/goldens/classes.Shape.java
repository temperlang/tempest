package classes;

public interface Shape {
    int getArea();

    default String describe() {
        return "shape of area " + this.getArea();
    }
}
