package libraries;

public final class Happened<T> implements Effect<T> {
    public final T event;

    public Happened(T event) {
        this.event = event;
    }

    public T getEvent() {
        return this.event;
    }
}
