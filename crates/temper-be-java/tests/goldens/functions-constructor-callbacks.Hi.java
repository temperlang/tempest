package constructor_callbacks;

import java.util.function.Function;

public final class Hi<T> {
    public final Function<T, T> ha;

    public Hi(Function<T, T> ha) {
        this.ha = ha;
    }

    public Function<T, T> getHa() {
        return this.ha;
    }
}
