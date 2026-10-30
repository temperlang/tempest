package call_override_from_subtype;

interface B3Deep extends B2Deep {
    default String a() {
        return B2Deep.super.a();
    }
}
