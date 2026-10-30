package call_override_from_subtype;

interface B2Deep extends B1Deep {
    default String a() {
        return B1Deep.super.a();
    }
}
