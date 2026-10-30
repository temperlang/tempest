package call_override_from_subtype;

interface B1Deep extends ADeep {
    default String a() {
        return "B1 wins!";
    }
}
