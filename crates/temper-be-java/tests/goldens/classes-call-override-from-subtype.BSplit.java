package call_override_from_subtype;

interface BSplit extends ASplit {
    default String a() {
        return ASplit.super.a();
    }
}
