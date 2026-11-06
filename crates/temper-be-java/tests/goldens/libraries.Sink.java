package libraries;

public interface Sink<T> {
    int take(T e);
}
