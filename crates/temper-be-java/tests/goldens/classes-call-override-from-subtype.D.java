package call_override_from_subtype;

final class D implements B, C {
    public D() {}

    public String a() {
        return C.super.a();
    }

    public String getThing() {
        return C.super.getThing();
    }
}
