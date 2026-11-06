package errors;

import temper.core.Core;

final class Box {
    public final int value;

    public Box(int value) {
        this.value = value;
    }

    public int get() {
        return this.value;
    }

    public String bad() {
        throw Core.bubble();
    }

    public int getValue() {
        return this.value;
    }
}
