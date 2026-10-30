package call_override_from_subtype;

final class DSplit implements BSplit, CSplit {
    public DSplit() {}

    public String a() {
        return CSplit.super.a();
    }
}
