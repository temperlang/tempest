package object_literals;

final class Whatever {
    public final String blah;

    public Whatever(int blech) {
        this.blah = Integer.toString(blech);
    }

    public String getBlah() {
        return this.blah;
    }
}
