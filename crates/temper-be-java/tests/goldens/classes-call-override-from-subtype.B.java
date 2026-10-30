package call_override_from_subtype;

interface B extends A {
    default String a() {
        return A.super.a();
    }

    default String getThing() {
        return A.super.getThing();
    }
}
