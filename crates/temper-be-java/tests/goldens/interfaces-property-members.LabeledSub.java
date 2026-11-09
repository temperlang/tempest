package property_members;

final class LabeledSub implements Labeled<String> {
    public final C thing;

    public LabeledSub(C thing) {
        this.thing = thing;
    }

    public String label(String l) {
        return l + ": " + this.thing.x;
    }

    public C getThing() {
        return this.thing;
    }
}
