package object_literals;

import object_literals.extra_class.Stringable;

final class Named implements Stringable {
    public final String name;

    public Named(String name) {
        this.name = name;
    }

    public String toString() {
        return "Named " + this.name;
    }

    public String getName() {
        return this.name;
    }
}
