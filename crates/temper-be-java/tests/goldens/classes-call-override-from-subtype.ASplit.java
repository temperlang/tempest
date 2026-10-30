package call_override_from_subtype;

interface ASplit {
    default String a() {
        return "A wins!";
    }
}
