package call_override_from_subtype;

interface C extends A {
    default String a() {
        return "C wins!";
    }

    default String getThing() {
        return "C has thing!";
    }
}
