package call_override_from_subtype;

interface A {
    default String a() {
        return "A wins!";
    }

    default String getThing() {
        return "A has thing!";
    }
}
