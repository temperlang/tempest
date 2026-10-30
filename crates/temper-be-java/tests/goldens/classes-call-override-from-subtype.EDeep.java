package call_override_from_subtype;

final class EDeep implements CDeep, B2Deep {
    public EDeep() {}

    public String a() {
        return CDeep.super.a();
    }
}
