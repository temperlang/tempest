package call_override_from_subtype;

interface CDeep extends ADeep {
    default String a() {
        return ADeep.super.a();
    }
}
