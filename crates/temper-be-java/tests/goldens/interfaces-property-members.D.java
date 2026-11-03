package property_members;

final class D<T extends I> {
    public final T thing;

    public D(T thing) {
        this.thing = thing;
    }

    public T getThing() {
        return this.thing;
    }
}
