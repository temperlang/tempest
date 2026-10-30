package java_classes;

public interface Polite extends Named {
    default String greet(int times) {
        return Named.super.greet(times);
    }
}
