package libraries;

public final class Ticks implements Counting<String> {
    private int n;

    public Ticks() {
        this.n = 0;
    }

    public int getCount() {
        return this.n;
    }

    public void setCount(int c) {
        this.n = c;
    }

    public String made() {
        return "tick" + this.n;
    }
}
