package java_classes;

public interface Named {
    int zero = 0;

    String getName();

    boolean isLoud();

    int getVolume();

    void setVolume(int v);

    default String greet(int times) {
        return JavaClassesGlobal.greeting + " " + this.getName() + " x" + times;
    }
}
