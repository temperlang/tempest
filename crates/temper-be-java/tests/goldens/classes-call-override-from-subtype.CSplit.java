package call_override_from_subtype;

interface CSplit {
    default String a() {
        return "C wins!";
    }
}
