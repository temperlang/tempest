package as_values;

import java.util.function.Consumer;

final class DelayedFunction {
    private final Consumer<String> f;

    public DelayedFunction(Consumer<String> f) {
        this.f = f;
    }

    public void callIt() {
        this.f.accept("from a method");
    }
}
