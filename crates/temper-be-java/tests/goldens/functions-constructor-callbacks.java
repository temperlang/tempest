package constructor_callbacks;

import java.util.function.Function;

public final class ConstructorCallbacksGlobal {
    private ConstructorCallbacksGlobal() {}

    public static <T> Hi<T> makeHi(Function<T, T> ha) {
        return new Hi<T>(ha);
    }

    public static Hi<Integer> newIntHi(int j) {
        return new Hi<Integer>(i -> i + j);
    }

    public static Hi<Integer> makeIntHi(int j) {
        return makeHi(i -> i + j);
    }
}
