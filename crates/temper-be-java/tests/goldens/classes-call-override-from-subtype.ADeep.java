package call_override_from_subtype;

interface ADeep {
    default String a() {
        return "A wins!";
    }
}
