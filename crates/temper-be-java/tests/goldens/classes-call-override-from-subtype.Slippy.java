package call_override_from_subtype;

final class Slippy implements Slappy {
    public Slippy() {}

    public boolean checkOverridden() {
        return this.getThingums() == 1;
    }

    public boolean checkOwn() {
        return this.getThingumses() == 2;
    }

    public int getThingums() {
        return 3;
    }

    public int getThingumses() {
        return 4;
    }
}
