package angle_call;

final class Hi {
    public final Blah<String> thing;

    public Hi(Blah<String> thing) {
        this.thing = thing;
    }

    public Blah<String> getThing() {
        return this.thing;
    }
}
