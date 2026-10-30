package call_override_from_subtype;

final class DDeep implements B2Deep, CDeep {
    public DDeep() {}

    public String a() {
        return B2Deep.super.a();
    }
}
