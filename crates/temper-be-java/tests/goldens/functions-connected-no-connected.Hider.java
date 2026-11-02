package work;

public final class Hider {
    private final int i;
    private final Hidden hidden;

    public Hider(int i) {
        this.i = i;
        this.hidden = new Hidden(this.i);
    }

    public int plus(int j) {
        return WorkGlobal.sum(this.hidden.i, j);
    }

    public int times(int j) {
        return WorkGlobal.prod(this.hidden, j);
    }
}
