package property_members;

final class SSub<T extends I> implements SI<T> {
    public final T thing;

    public SSub(T thing) {
        this.thing = thing;
    }

    public T getThing() {
        return this.thing;
    }
}
